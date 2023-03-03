-- MariaDB dump 10.19  Distrib 10.10.3-MariaDB, for Linux (x86_64)
--
-- Host: 127.0.0.1    Database: db
-- ------------------------------------------------------
-- Server version	8.0.32

/*!40101 SET @OLD_CHARACTER_SET_CLIENT=@@CHARACTER_SET_CLIENT */;
/*!40101 SET @OLD_CHARACTER_SET_RESULTS=@@CHARACTER_SET_RESULTS */;
/*!40101 SET @OLD_COLLATION_CONNECTION=@@COLLATION_CONNECTION */;
/*!40101 SET NAMES utf8mb4 */;
/*!40103 SET @OLD_TIME_ZONE=@@TIME_ZONE */;
/*!40103 SET TIME_ZONE='+00:00' */;
/*!40014 SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0 */;
/*!40014 SET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0 */;
/*!40101 SET @OLD_SQL_MODE=@@SQL_MODE, SQL_MODE='NO_AUTO_VALUE_ON_ZERO' */;
/*!40111 SET @OLD_SQL_NOTES=@@SQL_NOTES, SQL_NOTES=0 */;

--
-- Table structure for table `ACCOUNTS`
--

DROP TABLE IF EXISTS `ACCOUNTS`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!40101 SET character_set_client = utf8 */;
CREATE TABLE `ACCOUNTS` (
  `id` bigint unsigned DEFAULT NULL,
  `email` varchar(255) DEFAULT NULL,
  `hash` varchar(86) DEFAULT NULL,
  `salt` char(32) DEFAULT NULL,
  `name` varchar(255) DEFAULT NULL,
  `num` smallint unsigned DEFAULT NULL
);
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `ACCOUNTS`
--

LOCK TABLES `ACCOUNTS` WRITE;
/*!40000 ALTER TABLE `ACCOUNTS` DISABLE KEYS */;
INSERT INTO `ACCOUNTS` VALUES
(0,'test@test.test','',NULL,'test',0),
(2,'verzamelmeel@gmail.com','El3LMKwC/MYrUX5G6G/FpZ2QfkR9C6u6mGyYmWy6wsI','4Uw0LLpam8hzCgphrbxX2BT3uTDgUmgU','EsfoNL',1),
(1,'test@gmail.com','7MhzPWJhwJdphFrQLTsPKnLEVHD9KHurbH+aL3U1yNE','bOEP34ftcZgiET0N2yIegLgsW0826uVb','EsfoNL',2),
(3,'test1@gmail.com','QD3xJ1RWlAkTba8aVZaUYXqI0KZyPB8ofwpR+UL//E8','DCdlRnD1SHEW3qlwXx5muQQpsMkboBsF','EsfoNL',3);
/*!40000 ALTER TABLE `ACCOUNTS` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `MESSAGES`
--

DROP TABLE IF EXISTS `MESSAGES`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!40101 SET character_set_client = utf8 */;
CREATE TABLE `MESSAGES` (
  `sender` bigint unsigned DEFAULT NULL,
  `reciever` bigint unsigned DEFAULT NULL,
  `message` varchar(255) DEFAULT NULL,
  `queuepos` smallint unsigned DEFAULT NULL
);
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `MESSAGES`
--

LOCK TABLES `MESSAGES` WRITE;
/*!40000 ALTER TABLE `MESSAGES` DISABLE KEYS */;
INSERT INTO `MESSAGES` VALUES
(NULL,NULL,NULL,NULL),
(3,2,'hello world!',0),
(3,2,'hello world!',0),
(3,2,'hello world!',0),
(3,2,'hello world!',1),
(3,2,'hello world!',2);
/*!40000 ALTER TABLE `MESSAGES` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `REFRESH_TOKENS`
--

DROP TABLE IF EXISTS `REFRESH_TOKENS`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!40101 SET character_set_client = utf8 */;
CREATE TABLE `REFRESH_TOKENS` (
  `id` bigint unsigned DEFAULT NULL,
  `refresh_token_hash` varchar(86) DEFAULT NULL,
  `salt` char(32) DEFAULT NULL,
  `refresh_token_expiry` timestamp NULL DEFAULT NULL
);
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `REFRESH_TOKENS`
--

LOCK TABLES `REFRESH_TOKENS` WRITE;
/*!40000 ALTER TABLE `REFRESH_TOKENS` DISABLE KEYS */;
INSERT INTO `REFRESH_TOKENS` VALUES
(2,'USachS+/bLq5IAnXRljA8pI0l0Oq238NghCF1YNKM4I','2rzgIqGhRm9xPfsgMSYEV2Pgn31Zz8q4','2023-03-18 21:37:04'),
(1,'1k3zRHXAxDCZIxB4caH3eKD/tTS95/hmB2/MRGyCZlQ','U2YaFFvlMkciGdHMfIVfIz6g5MzIdnXv','2023-03-18 21:37:33'),
(3,'BSxw2QY+P2XOZ0rh8+a7LtITSAEcCKiZNQ95umGFdaI','6NF8gzPjJi0X2TpRJE2loZHftkIqtnKG','2023-03-18 21:38:18');
/*!40000 ALTER TABLE `REFRESH_TOKENS` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `TOKENS`
--

DROP TABLE IF EXISTS `TOKENS`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!40101 SET character_set_client = utf8 */;
CREATE TABLE `TOKENS` (
  `id` bigint unsigned DEFAULT NULL,
  `token_hash` varchar(86) DEFAULT NULL,
  `salt` char(32) DEFAULT NULL,
  `token_expiry` timestamp NULL DEFAULT NULL
);
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `TOKENS`
--

LOCK TABLES `TOKENS` WRITE;
/*!40000 ALTER TABLE `TOKENS` DISABLE KEYS */;
INSERT INTO `TOKENS` VALUES
(2,'LjmwblxwIVehnIDoO0ktaMgcoMsZJE1anZBi1ATaw8E','xEf4gouOICWBeiCAAOqDDngWwVEY6u5p','2023-02-23 21:37:04'),
(1,'Kz/FN/1+QueK/2YsAjeUTXK/s1r1YDWwJAj0ylbZa/0','uoL9hRxSJB94lEIJaelOQ0Vt2EFs9C45','2023-02-23 21:37:33'),
(3,'Ik5wtNuVs6p4kQ2qRGbzFA1nRj63ABIGmGbO0zQc/J0','GHSsUP8iiiILG8ocvTOseIxoii4KgNhn','2023-02-23 21:38:18');
/*!40000 ALTER TABLE `TOKENS` ENABLE KEYS */;
UNLOCK TABLES;
/*!40103 SET TIME_ZONE=@OLD_TIME_ZONE */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;

-- Dump completed on 2023-02-17 10:50:05
