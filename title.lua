function Writer(doc, _)
  -- print((nil).text);
  if doc.meta.title then
    return doc.meta.title[1].text
  else
    return ""
  end
end
