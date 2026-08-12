use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    io::{Seek, Write},
    os::unix::fs::FileExt,
    path::PathBuf,
};

use axum::{
    extract::{Path, Request},
    routing::post,
    Router,
};
use axum_extra::TypedHeader;
use headers_core::Header;
use http::{HeaderName, StatusCode};
use std::fs::File;
use tokio::{sync::Mutex, task::JoinHandle};
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::{admin::AuthUser, api::filestream, prelude::*};
use axum::extract::State as AState;

pub struct ByteOffset(u64);
static BYTE_OFFSET_HEADERNAME: HeaderName = HeaderName::from_static("byte-offset");
impl Header for ByteOffset {
    fn name() -> &'static http::HeaderName {
        &BYTE_OFFSET_HEADERNAME
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers_core::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i http::HeaderValue>,
    {
        Ok(ByteOffset(
            values
                .last()
                .ok_or(headers_core::Error::invalid())?
                .to_str()
                .map_err(|_| headers_core::Error::invalid())
                .and_then(|e| e.parse().map_err(|_| headers_core::Error::invalid()))?,
        ))
    }

    fn encode<E: Extend<http::HeaderValue>>(&self, values: &mut E) {
        values.extend(http::HeaderValue::from_str(&format!("{}", self.0)));
    }
}

#[derive(Debug)]
pub struct FileStream {
    file: Arc<Mutex<File>>,
    path: PathBuf,
}

impl FileStream {
    pub async fn finish(self) -> PathBuf {
        let _ = self.file.lock().await.sync_all();
        self.path
    }
}

// const TTL: tokio::time::Duration = tokio::time::Duration::from_mins(10);
const TTL: tokio::time::Duration = tokio::time::Duration::from_mins(1);

/// POST
// #[axum::debug_handler]
#[tracing::instrument(skip(_auth, state))]
pub async fn create_upload(
    location: &std::path::Path,
    AState(state): AState<Arc<State>>,
    _auth: AuthUser,
    Path(filename): Path<String>,
) -> Result<String, StatusCode> {
    debug!("upload created");
    let uuid = Uuid::new_v4();
    let filename_path = PathBuf::from(&filename);
    let filename_basename = filename_path.file_name().ok_or(StatusCode::BAD_REQUEST)?;
    if filename_basename != filename.as_str() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut path = location.to_path_buf();
    path.push(filename);
    let file = Arc::new(
        File::create(path.clone())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .into(),
    );

    state
        .filestreams
        .insert(uuid.clone(), FileStream { file, path })
        .await;

    Ok(uuid.to_string())
}

/// POST
pub async fn upload_chunk(
    AState(state): AState<Arc<State>>,
    _auth: AuthUser,
    Path(uuid): Path<Uuid>,
    TypedHeader(offset): TypedHeader<ByteOffset>,
    body: Request,
) -> Result<(), StatusCode> {
    let filestream = state
        .filestreams
        .get(&uuid)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut stream = body.into_body().into_data_stream();
    let file = filestream.file.clone();
    let mut lock = file.lock_owned().await;
    lock.seek(std::io::SeekFrom::Start(offset.0))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        // let file = lock.clone();
        lock = tokio::task::spawn_blocking(move || {
            // let lock = file.blocking_lock();
            lock.write_all(&chunk)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(lock)
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .flatten()?;
    }

    Ok(())
}

/// POST
/// handler gets called with hopefully completed file at filepath
pub async fn finish_upload<Fu: Future<Output = ()>, F: Fn(std::path::PathBuf, Arc<State>) -> Fu>(
    handler: F,
    AState(state): AState<Arc<State>>,
    _auth: AuthUser,
    Path(uuid): Path<Uuid>,
) -> Result<(), StatusCode> {
    let filepath = state
        .filestreams
        .remove(&uuid)
        .await
        .ok_or(StatusCode::NOT_FOUND)?
        .finish()
        .await;
    debug!("file {filepath:?} finished");
    handler(filepath, state.clone()).await;
    Ok(())
}

pub fn file_uploader<
    Fu: Future<Output = ()> + 'static + Send,
    F: Fn(std::path::PathBuf, Arc<State>) -> Fu + Send + Sync + 'static + Clone,
>(
    output_path: PathBuf,
    file_handler: F,
) -> Router<Arc<State>> {
    // let file_handler_clone = file_handler.clone();
    let finish_upload_handler =
        async move |state: AState<Arc<State>>, auth: AuthUser, uuid: Path<Uuid>| {
            let handler = file_handler.clone();
            finish_upload(&handler, state, auth, uuid).await
            // todo!()
        }; // as AsyncFn(),
    Router::new()
        .route(
            "/create/{*filename}",
            post(async move |state, auth, filename| {
                let output_path = output_path;
                create_upload(&output_path, state, auth, filename).await
            }),
        )
        .route("/chunk/{*uuid}", post(upload_chunk))
        .route("/finish/{*uuid}", post(finish_upload_handler))
}

#[derive(Debug)]
pub(crate) struct FileStreams {
    inner: Arc<Mutex<InnerFileStreams>>,
    task: Mutex<JoinHandle<()>>,
}

#[derive(Debug)]
struct InnerFileStreams {
    streams: HashMap<Uuid, filestream::FileStream>,
    ttlqueue: VecDeque<(tokio::time::Instant, Uuid)>,
}

impl std::default::Default for FileStreams {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            task: Mutex::new(tokio::spawn(async {})),
        }
    }
}

impl std::default::Default for InnerFileStreams {
    fn default() -> Self {
        Self {
            ttlqueue: Default::default(),
            streams: Default::default(),
        }
    }
}

// struct FileStreamRef {
//     name: tokio::sync::MutexGuard<'s, >}

impl FileStreams {
    async fn insert(&self, uuid: Uuid, filestream: FileStream) {
        let mut lock = self.inner.lock().await;
        if lock.insert(uuid, filestream).await {
            self.refresh_task().await;
        };
    }

    async fn get<'s>(
        &'s self,
        uuid: &Uuid,
    ) -> Option<tokio::sync::MappedMutexGuard<'s, FileStream>> {
        let out =
            tokio::sync::MutexGuard::try_map(self.inner.lock().await, |e| e.streams.get_mut(uuid));

        out.ok()
    }

    async fn remove(&self, uuid: &Uuid) -> Option<FileStream> {
        self.inner.lock().await.streams.remove(uuid)
    }

    async fn refresh_task(&self) {
        let mut lock = self.task.lock().await;
        lock.abort();
        let inner = self.inner.clone();
        *lock = tokio::spawn(async move {
            while let Some((ttl, uuid)) = {
                let lock = inner.lock().await;
                let res = lock.ttlqueue.front();
                res.map(ToOwned::to_owned)
            } {
                debug!("waiting until {:#?}", ttl);
                tokio::time::sleep_until(ttl.clone()).await;
                let mut lock = inner.lock().await;
                assert!(lock.ttlqueue.front().is_some());
                assert_eq!(lock.ttlqueue.pop_front().unwrap(), (ttl, uuid));
                // all consuming actions remove the stream from the [InnerFileStreams.streams]
                if let Some(stream) = lock.streams.remove(&uuid) {
                    debug!("removing file {:?}", stream.path);
                    let _ = tokio::fs::remove_file(stream.path).await;
                }
            }
        })
    }
}

impl InnerFileStreams {
    /// returns true if the queue was empty
    async fn insert(&mut self, uuid: Uuid, filestream: FileStream) -> bool {
        self.streams.insert(uuid.clone(), filestream);
        let ret = self.ttlqueue.len() == 0;
        self.ttlqueue
            .push_back((tokio::time::Instant::now() + TTL, uuid));
        ret
    }
}
