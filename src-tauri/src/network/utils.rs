use std::net::IpAddr;
use std::str::FromStr;

use crate::network::types::{NetworkError, Result};

const OUI_VENDORS: &[(&str, &str)] = &[
    ("00:50:56", "VMware"),
    ("00:0c:29", "VMware"),
    ("00:1c:14", "VMware"),
    ("08:00:27", "VirtualBox"),
    ("0a:00:27", "VirtualBox"),
    ("52:54:00", "QEMU"),
    ("00:16:3e", "Xen"),
    ("00:15:5d", "Microsoft Hyper-V"),
    ("00:1b:21", "Intel"),
    ("00:1c:c4", "Intel"),
    ("00:21:5c", "Intel"),
    ("00:22:fa", "Intel"),
    ("00:23:14", "Intel"),
    ("00:24:d6", "Intel"),
    ("00:26:c6", "Intel"),
    ("00:1c:42", "Parallels"),
    ("00:25:bc", "Apple"),
    ("00:3e:e1", "Apple"),
    ("00:56:cd", "Apple"),
    ("00:61:71", "Apple"),
    ("00:90:27", "Apple"),
    ("00:a0:40", "Apple"),
    ("00:b0:52", "Apple"),
    ("00:c0:8c", "Apple"),
    ("00:c3:a8", "Apple"),
    ("00:cd:fe", "Apple"),
    ("00:db:70", "Apple"),
    ("00:e0:b0", "Apple"),
    ("00:ea:4c", "Apple"),
    ("04:0c:ce", "Apple"),
    ("04:1e:64", "Apple"),
    ("04:26:65", "Apple"),
    ("04:48:c9", "Apple"),
    ("04:4b:ed", "Apple"),
    ("04:52:f3", "Apple"),
    ("04:69:f8", "Apple"),
    ("04:8b:42", "Apple"),
    ("04:c8:07", "Apple"),
    ("04:d3:b5", "Apple"),
    ("04:d7:95", "Apple"),
    ("04:e5:36", "Apple"),
    ("04:f1:3e", "Apple"),
    ("08:00:07", "Apple"),
    ("08:66:98", "Apple"),
    ("08:6d:41", "Apple"),
    ("08:71:90", "Apple"),
    ("08:c2:1a", "Apple"),
    ("08:ec:a9", "Apple"),
    ("08:f8:bc", "Apple"),
    ("0c:15:39", "Apple"),
    ("0c:3e:9f", "Apple"),
    ("0c:4d:e9", "Apple"),
    ("0c:74:c2", "Apple"),
    ("0c:77:1a", "Apple"),
    ("0c:96:e6", "Apple"),
    ("0c:9a:3c", "Apple"),
    ("0c:bc:af", "Apple"),
    ("0c:c4:11", "Apple"),
    ("0c:d5:02", "Apple"),
    ("10:00:81", "Apple"),
    ("10:1c:0c", "Apple"),
    ("10:41:7f", "Apple"),
    ("10:44:7a", "Apple"),
    ("10:51:07", "Apple"),
    ("10:5f:06", "Apple"),
    ("10:93:e9", "Apple"),
    ("10:9f:a9", "Apple"),
    ("10:a5:9e", "Apple"),
    ("10:ad:a1", "Apple"),
    ("10:b0:81", "Apple"),
    ("10:bf:48", "Apple"),
    ("10:c5:95", "Apple"),
    ("10:ce:a9", "Apple"),
    ("10:d5:42", "Apple"),
    ("10:dd:b1", "Apple"),
    ("10:e6:ae", "Apple"),
    ("14:10:9f", "Apple"),
    ("14:20:5e", "Apple"),
    ("14:5a:05", "Apple"),
    ("14:7d:c5", "Apple"),
    ("14:98:77", "Apple"),
    ("14:9f:3c", "Apple"),
    ("14:a7:2b", "Apple"),
    ("14:c2:13", "Apple"),
    ("18:00:2b", "Apple"),
    ("18:20:32", "Apple"),
    ("18:65:90", "Apple"),
    ("18:81:de", "Apple"),
    ("18:af:61", "Apple"),
    ("18:e7:f4", "Apple"),
    ("18:f6:43", "Apple"),
    ("1c:36:bb", "Apple"),
    ("1c:5c:42", "Apple"),
    ("1c:7d:22", "Apple"),
    ("1c:98:5b", "Apple"),
    ("1c:9e:46", "Apple"),
    ("1c:ab:a7", "Apple"),
    ("1c:b0:0c", "Apple"),
    ("1c:b3:c9", "Apple"),
    ("1c:e6:2b", "Apple"),
    ("1c:f1:03", "Apple"),
    ("20:a5:cb", "Apple"),
    ("20:c3:8f", "Apple"),
    ("24:a0:74", "Apple"),
    ("24:ab:81", "Apple"),
    ("24:db:ed", "Apple"),
    ("28:37:37", "Apple"),
    ("28:5f:db", "Apple"),
    ("28:6a:ba", "Apple"),
    ("28:6a:ea", "Apple"),
    ("28:6a:f7", "Apple"),
    ("28:95:5c", "Apple"),
    ("28:9a:4b", "Apple"),
    ("28:be:43", "Apple"),
    ("28:cf:05", "Apple"),
    ("28:d9:8a", "Apple"),
    ("28:e0:2c", "Apple"),
    ("28:e1:4c", "Apple"),
    ("28:ec:95", "Apple"),
    ("28:f0:76", "Apple"),
    ("28:f3:66", "Apple"),
    ("28:fe:98", "Apple"),
    ("2c:1f:23", "Apple"),
    ("2c:32:5e", "Apple"),
    ("2c:33:11", "Apple"),
    ("2c:b4:3a", "Apple"),
    ("2c:f0:ee", "Apple"),
    ("30:10:e6", "Apple"),
    ("30:19:66", "Apple"),
    ("30:35:ad", "Apple"),
    ("30:3a:64", "Apple"),
    ("30:42:40", "Apple"),
    ("30:63:6b", "Apple"),
    ("30:90:ab", "Apple"),
    ("30:d1:7e", "Apple"),
    ("30:f7:c5", "Apple"),
    ("34:08:bc", "Apple"),
    ("34:12:88", "Apple"),
    ("34:36:3b", "Apple"),
    ("34:51:aa", "Apple"),
    ("34:95:db", "Apple"),
    ("34:c3:ac", "Apple"),
    ("34:e2:fd", "Apple"),
    ("34:e3:94", "Apple"),
    ("38:0f:4a", "Apple"),
    ("38:37:8b", "Apple"),
    ("38:48:4c", "Apple"),
    ("38:59:f9", "Apple"),
    ("38:6e:88", "Apple"),
    ("38:71:de", "Apple"),
    ("38:c7:0a", "Apple"),
    ("38:c9:86", "Apple"),
    ("38:ca:84", "Apple"),
    ("38:f9:d3", "Apple"),
    ("3c:07:54", "Apple"),
    ("3c:2e:ff", "Apple"),
    ("3c:5a:b4", "Apple"),
    ("3c:61:0b", "Apple"),
    ("3c:62:00", "Apple"),
    ("3c:7d:0a", "Apple"),
    ("3c:a6:f6", "Apple"),
    ("3c:b8:7a", "Apple"),
    ("3c:d0:bd", "Apple"),
    ("3c:e0:72", "Apple"),
    ("3c:f7:a4", "Apple"),
    ("3c:f8:62", "Apple"),
    ("40:00:03", "Apple"),
    ("40:3c:fc", "Apple"),
    ("40:6c:8f", "Apple"),
    ("40:82:7a", "Apple"),
    ("40:9c:28", "Apple"),
    ("40:a6:d9", "Apple"),
    ("40:b3:95", "Apple"),
    ("40:b6:b1", "Apple"),
    ("40:cb:c0", "Apple"),
    ("40:d3:2d", "Apple"),
    ("40:d5:82", "Apple"),
    ("40:f3:08", "Apple"),
    ("40:fa:be", "Apple"),
    ("44:00:10", "Apple"),
    ("44:2a:60", "Apple"),
    ("44:4c:0b", "Apple"),
    ("44:52:2a", "Apple"),
    ("44:94:bb", "Apple"),
    ("44:98:5d", "Apple"),
    ("44:d8:78", "Apple"),
    ("44:f4:7d", "Apple"),
    ("48:43:7c", "Apple"),
    ("48:60:5f", "Apple"),
    ("48:74:6e", "Apple"),
    ("48:89:d0", "Apple"),
    ("48:9c:03", "Apple"),
    ("48:a1:95", "Apple"),
    ("48:b4:6b", "Apple"),
    ("48:bf:74", "Apple"),
    ("48:bf:6b", "Apple"),
    ("48:c7:96", "Apple"),
    ("48:d6:2b", "Apple"),
    ("48:ea:63", "Apple"),
    ("48:f0:7b", "Apple"),
    ("4c:32:75", "Apple"),
    ("4c:57:60", "Apple"),
    ("4c:8d:79", "Apple"),
    ("4c:98:80", "Apple"),
    ("4c:9d:ed", "Apple"),
    ("4c:af:b4", "Apple"),
    ("4c:bc:48", "Apple"),
    ("50:1d:93", "Apple"),
    ("50:32:37", "Apple"),
    ("50:7a:55", "Apple"),
    ("50:82:d5", "Apple"),
    ("50:84:74", "Apple"),
    ("50:92:b9", "Apple"),
    ("50:9f:27", "Apple"),
    ("50:a6:7f", "Apple"),
    ("50:bc:96", "Apple"),
    ("50:c8:e5", "Apple"),
    ("50:d3:bc", "Apple"),
    ("50:de:06", "Apple"),
    ("50:e5:49", "Apple"),
    ("50:eb:71", "Apple"),
    ("54:1d:c2", "Apple"),
    ("54:26:96", "Apple"),
    ("54:2a:a2", "Apple"),
    ("54:33:cb", "Apple"),
    ("54:4e:90", "Apple"),
    ("54:72:4f", "Apple"),
    ("54:9f:13", "Apple"),
    ("54:ae:14", "Apple"),
    ("54:e4:3a", "Apple"),
    ("54:ea:a8", "Apple"),
    ("58:1f:28", "Apple"),
    ("58:40:4e", "Apple"),
    ("58:7f:66", "Apple"),
    ("58:96:1d", "Apple"),
    ("58:b1:5f", "Apple"),
    ("58:b0:35", "Apple"),
    ("58:b3:fc", "Apple"),
    ("58:b9:e1", "Apple"),
    ("58:cf:4b", "Apple"),
    ("58:d3:49", "Apple"),
    ("58:d9:c6", "Apple"),
    ("58:e6:36", "Apple"),
    ("58:f0:8f", "Apple"),
    ("5c:3c:27", "Apple"),
    ("5c:57:1a", "Apple"),
    ("5c:59:48", "Apple"),
    ("5c:8d:4e", "Apple"),
    ("5c:95:ae", "Apple"),
    ("5c:96:9d", "Apple"),
    ("5c:b8:24", "Apple"),
    ("5c:e8:6e", "Apple"),
    ("5c:f9:38", "Apple"),
    ("60:03:08", "Apple"),
    ("60:30:d4", "Apple"),
    ("60:33:4b", "Apple"),
    ("60:41:5c", "Apple"),
    ("60:57:18", "Apple"),
    ("60:92:17", "Apple"),
    ("60:93:84", "Apple"),
    ("60:a3:7d", "Apple"),
    ("60:ab:67", "Apple"),
    ("60:ad:0e", "Apple"),
    ("60:b0:34", "Apple"),
    ("60:c5:e6", "Apple"),
    ("60:d9:c7", "Apple"),
    ("60:e8:3b", "Apple"),
    ("60:f4:38", "Apple"),
    ("60:fa:ad", "Apple"),
    ("60:fb:42", "Apple"),
    ("60:fe:20", "Apple"),
    ("64:20:0c", "Apple"),
    ("64:5a:ed", "Apple"),
    ("64:70:02", "Apple"),
    ("64:9c:81", "Apple"),
    ("64:a3:cb", "Apple"),
    ("64:a5:c3", "Apple"),
    ("64:a6:51", "Apple"),
    ("64:b0:a8", "Apple"),
    ("64:b8:29", "Apple"),
    ("64:b9:e8", "Apple"),
    ("64:c2:10", "Apple"),
    ("64:c3:5c", "Apple"),
    ("64:d4:bd", "Apple"),
    ("64:e6:82", "Apple"),
    ("64:eb:8c", "Apple"),
    ("68:5b:35", "Apple"),
    ("68:96:7b", "Apple"),
    ("68:9c:70", "Apple"),
    ("68:9c:5e", "Apple"),
    ("68:9d:0d", "Apple"),
    ("68:a8:6d", "Apple"),
    ("68:ae:20", "Apple"),
    ("68:c4:4d", "Apple"),
    ("68:d7:9a", "Apple"),
    ("68:db:67", "Apple"),
    ("68:f7:28", "Apple"),
    ("68:fb:7e", "Apple"),
    ("6c:3e:6d", "Apple"),
    ("6c:40:08", "Apple"),
    ("6c:70:9f", "Apple"),
    ("6c:72:20", "Apple"),
    ("6c:96:cf", "Apple"),
    ("6c:ab:31", "Apple"),
    ("6c:ad:ef", "Apple"),
    ("6c:c2:6a", "Apple"),
    ("6c:c3:74", "Apple"),
    ("6c:d0:32", "Apple"),
    ("6c:e0:30", "Apple"),
    ("6c:e4:d4", "Apple"),
    ("70:14:a6", "Apple"),
    ("70:56:81", "Apple"),
    ("70:73:cb", "Apple"),
    ("70:81:e5", "Apple"),
    ("70:8a:09", "Apple"),
    ("70:97:92", "Apple"),
    ("70:a2:67", "Apple"),
    ("70:a5:84", "Apple"),
    ("70:ca:04", "Apple"),
    ("70:d3:79", "Apple"),
    ("70:de:e2", "Apple"),
    ("70:e7:2c", "Apple"),
    ("74:04:f0", "Apple"),
    ("74:1b:b2", "Apple"),
    ("74:2d:0f", "Apple"),
    ("74:5c:9f", "Apple"),
    ("74:81:14", "Apple"),
    ("74:88:c8", "Apple"),
    ("74:8c:54", "Apple"),
    ("74:95:ec", "Apple"),
    ("74:b8:57", "Apple"),
    ("74:bf:c0", "Apple"),
    ("74:c7:46", "Apple"),
    ("74:e1:b6", "Apple"),
    ("74:e5:43", "Apple"),
    ("78:31:c1", "Apple"),
    ("78:3a:67", "Apple"),
    ("78:4f:43", "Apple"),
    ("78:4f:a0", "Apple"),
    ("78:64:3c", "Apple"),
    ("78:6c:1c", "Apple"),
    ("78:87:c2", "Apple"),
    ("78:a3:e4", "Apple"),
    ("78:ab:bb", "Apple"),
    ("78:b8:97", "Apple"),
    ("78:ca:39", "Apple"),
    ("78:d4:f1", "Apple"),
    ("78:db:2f", "Apple"),
    ("78:f5:57", "Apple"),
    ("78:f7:be", "Apple"),
    ("78:f8:82", "Apple"),
    ("7c:03:d6", "Apple"),
    ("7c:50:49", "Apple"),
    ("7c:6d:61", "Apple"),
    ("7c:7d:3d", "Apple"),
    ("7c:8f:e4", "Apple"),
    ("7c:c3:a1", "Apple"),
    ("7c:c5:37", "Apple"),
    ("7c:cf:25", "Apple"),
    ("7c:d1:66", "Apple"),
    ("7c:d6:61", "Apple"),
    ("7c:d9:90", "Apple"),
    ("7c:f0:ba", "Apple"),
    ("80:00:6e", "Apple"),
    ("80:02:18", "Apple"),
    ("80:13:82", "Apple"),
    ("80:29:94", "Apple"),
    ("80:49:38", "Apple"),
    ("80:4a:14", "Apple"),
    ("80:59:94", "Apple"),
    ("80:65:7f", "Apple"),
    ("80:6d:cb", "Apple"),
    ("80:82:f5", "Apple"),
    ("80:98:44", "Apple"),
    ("80:a1:ab", "Apple"),
    ("80:b0:52", "Apple"),
    ("80:bd:76", "Apple"),
    ("80:c5:f2", "Apple"),
    ("80:c6:ab", "Apple"),
    ("80:e4:82", "Apple"),
    ("80:ed:02", "Apple"),
    ("84:29:99", "Apple"),
    ("84:38:35", "Apple"),
    ("84:3d:c6", "Apple"),
    ("84:50:19", "Apple"),
    ("84:89:ad", "Apple"),
    ("84:89:ee", "Apple"),
    ("84:8d:4e", "Apple"),
    ("84:96:e3", "Apple"),
    ("84:b1:72", "Apple"),
    ("84:b2:1e", "Apple"),
    ("84:38:63", "Samsung"),
    ("84:8e:0c", "Apple"),
    ("84:a1:34", "Apple"),
    ("84:a8:a4", "Apple"),
    ("84:ab:1a", "Apple"),
    ("84:b1:53", "Apple"),
    ("84:b6:7e", "Apple"),
    ("84:ba:3b", "Apple"),
    ("84:be:68", "Apple"),
    ("84:c0:ef", "Apple"),
    ("84:cf:09", "Apple"),
    ("84:d3:28", "Apple"),
    ("84:d8:1b", "Apple"),
    ("84:db:2f", "Apple"),
    ("84:e1:4c", "Apple"),
    ("88:1b:3e", "Apple"),
    ("88:3b:62", "Apple"),
    ("88:4b:39", "Apple"),
    ("88:63:df", "Apple"),
    ("88:66:39", "Apple"),
    ("88:7d:12", "Apple"),
    ("88:8d:79", "Apple"),
    ("88:98:21", "Apple"),
    ("88:9b:39", "Apple"),
    ("88:a5:ba", "Apple"),
    ("88:ad:43", "Apple"),
    ("88:b1:14", "Apple"),
    ("88:b9:52", "Apple"),
    ("88:c3:97", "Apple"),
    ("88:cb:87", "Apple"),
    ("88:c9:6f", "Apple"),
    ("88:d7:bc", "Apple"),
    ("88:e9:fe", "Apple"),
    ("8c:00:6d", "Apple"),
    ("8c:2d:aa", "Apple"),
    ("8c:58:77", "Apple"),
    ("8c:5e:15", "Apple"),
    ("8c:7a:15", "Apple"),
    ("8c:7b:9d", "Apple"),
    ("8c:85:90", "Apple"),
    ("8c:8e:87", "Apple"),
    ("8c:94:36", "Apple"),
    ("8c:9d:27", "Apple"),
    ("8c:a9:cf", "Apple"),
    ("8c:ad:26", "Apple"),
    ("8c:ae:4c", "Apple"),
    ("8c:bc:20", "Apple"),
    ("8c:c7:ac", "Apple"),
    ("8c:cd:56", "Apple"),
    ("8c:ee:fb", "Apple"),
    ("90:18:7c", "Apple"),
    ("90:27:e4", "Apple"),
    ("90:2c:e2", "Apple"),
    ("90:48:27", "Apple"),
    ("90:72:40", "Apple"),
    ("90:84:0d", "Apple"),
    ("90:8d:6c", "Apple"),
    ("90:b0:ed", "Apple"),
    ("90:b1:1c", "Apple"),
    ("90:b2:1c", "Apple"),
    ("90:c1:c6", "Apple"),
    ("90:d8:2e", "Apple"),
    ("90:ad:d4", "Amazon"),
    ("90:e0:ab", "Apple"),
    ("90:fb:46", "Apple"),
    ("94:94:26", "Apple"),
    ("94:94:97", "Apple"),
    ("94:98:6f", "Apple"),
    ("94:9a:a4", "Apple"),
    ("94:9c:2b", "Apple"),
    ("94:9e:ba", "Apple"),
    ("94:ac:0b", "Apple"),
    ("94:b8:2d", "Apple"),
    ("94:e9:6a", "Apple"),
    ("94:f6:65", "Apple"),
    ("98:00:c6", "Apple"),
    ("98:02:d8", "Apple"),
    ("98:03:d8", "Apple"),
    ("98:06:4c", "Apple"),
    ("98:09:2e", "Apple"),
    ("98:0d:67", "Apple"),
    ("98:14:62", "Apple"),
    ("98:3d:fa", "Apple"),
    ("98:46:0a", "Apple"),
    ("98:4b:4a", "Apple"),
    ("98:4f:ee", "Apple"),
    ("98:5f:d3", "Apple"),
    ("98:6b:ed", "Apple"),
    ("98:6f:30", "Apple"),
    ("98:8b:5a", "Apple"),
    ("98:a8:e5", "Apple"),
    ("98:b8:bc", "Apple"),
    ("98:c6:1c", "Apple"),
    ("98:d2:93", "Apple"),
    ("98:d6:bb", "Apple"),
    ("98:de:ba", "Apple"),
    ("98:e0:7a", "Apple"),
    ("98:e6:3a", "Apple"),
    ("98:e7:43", "Apple"),
    ("98:f1:70", "Apple"),
    ("98:f2:21", "Apple"),
    ("98:f3:33", "Apple"),
    ("9c:04:ef", "Apple"),
    ("9c:20:ed", "Apple"),
    ("9c:2a:83", "Apple"),
    ("9c:2e:7a", "Apple"),
    ("9c:35:23", "Apple"),
    ("9c:4f:5f", "Apple"),
    ("9c:6b:72", "Apple"),
    ("9c:8e:cd", "Apple"),
    ("9c:99:a0", "Apple"),
    ("9c:9d:7e", "Apple"),
    ("9c:ae:d3", "Apple"),
    ("9c:b2:b2", "Apple"),
    ("9c:f4:8e", "Apple"),
    ("9c:f5:96", "Apple"),
    ("9c:fc:01", "Apple"),
    ("a0:18:ed", "Apple"),
    ("a0:3d:05", "Apple"),
    ("a0:4e:90", "Apple"),
    ("a0:72:91", "Apple"),
    ("a0:81:b7", "Apple"),
    ("a0:88:9b", "Apple"),
    ("a0:99:9b", "Apple"),
    ("a0:99:9b", "Apple"),
    ("a0:a4:c2", "Apple"),
    ("a0:a5:8f", "Apple"),
    ("a0:aa:bb", "Apple"),
    ("a0:b4:c8", "Apple"),
    ("a0:be:d4", "Apple"),
    ("a0:ce:c8", "Apple"),
    ("a0:d3:7a", "Apple"),
    ("a0:db:33", "Apple"),
    ("a0:dd:3c", "Apple"),
    ("a0:f3:c1", "Apple"),
    ("a4:18:c6", "Apple"),
    ("a4:26:79", "Apple"),
    ("a4:45:19", "Apple"),
    ("a4:4e:31", "Apple"),
    ("a4:53:0e", "Apple"),
    ("a4:5a:7e", "Apple"),
    ("a4:67:06", "Apple"),
    ("a4:67:af", "Apple"),
    ("a4:8c:db", "Apple"),
    ("a4:99:12", "Apple"),
    ("a4:99:47", "Apple"),
    ("a4:9b:34", "Apple"),
    ("a4:b8:05", "Apple"),
    ("a4:b8:05", "Apple"),
    ("a4:c3:61", "Apple"),
    ("a4:c4:94", "Apple"),
    ("a4:ce:15", "Apple"),
    ("a4:da:3c", "Apple"),
    ("a4:e0:68", "Apple"),
    ("a4:f1:e8", "Apple"),
    ("a4:f5:1a", "Apple"),
    ("a8:20:66", "Apple"),
    ("a8:3b:76", "Apple"),
    ("a8:5b:b5", "Apple"),
    ("a8:5c:2c", "Apple"),
    ("a8:5d:9c", "Apple"),
    ("a8:6d:aa", "Apple"),
    ("a8:79:8d", "Apple"),
    ("a8:86:dd", "Apple"),
    ("a8:8e:24", "Apple"),
    ("a8:8f:6a", "Apple"),
    ("a8:96:cf", "Apple"),
    ("a8:9a:93", "Apple"),
    ("a8:9a:e0", "Apple"),
    ("a8:a2:66", "Apple"),
    ("a8:b8:6e", "Apple"),
    ("a8:bc:9d", "Apple"),
    ("a8:c2:87", "Apple"),
    ("a8:cb:23", "Apple"),
    ("a8:cf:7c", "Apple"),
    ("a8:d0:0f", "Apple"),
    ("a8:d5:6c", "Apple"),
    ("a8:e0:61", "Apple"),
    ("a8:e0:7e", "Apple"),
    ("a8:e3:ee", "Apple"),
    ("a8:fa:26", "Apple"),
    ("ac:1f:6b", "TP-Link"),
    ("ac:1f:6d", "TP-Link"),
    ("ac:3c:0b", "Apple"),
    ("ac:44:f2", "Apple"),
    ("ac:51:35", "Apple"),
    ("ac:5f:3e", "Apple"),
    ("ac:61:ea", "Apple"),
    ("ac:66:be", "Apple"),
    ("ac:71:e9", "Apple"),
    ("ac:7f:3e", "Apple"),
    ("ac:88:fd", "Apple"),
    ("ac:91:9b", "Apple"),
    ("ac:9a:22", "Apple"),
    ("ac:a4:18", "Apple"),
    ("ac:b5:7d", "Apple"),
    ("ac:bc:32", "Apple"),
    ("ac:bc:ff", "Apple"),
    ("ac:cf:5c", "Apple"),
    ("ac:de:48", "Apple"),
    ("ac:e0:10", "Apple"),
    ("ac:e9:17", "Apple"),
    ("b0:02:47", "Apple"),
    ("b0:03:66", "Apple"),
    ("b0:06:3a", "Apple"),
    ("b0:10:41", "Apple"),
    ("b0:1f:7b", "Apple"),
    ("b0:2a:43", "Apple"),
    ("b0:34:95", "Apple"),
    ("b0:3d:4b", "Apple"),
    ("b0:48:1a", "Apple"),
    ("b0:4a:39", "Apple"),
    ("b0:65:bd", "Apple"),
    ("b0:70:2d", "Apple"),
    ("b0:7d:51", "Apple"),
    ("b0:82:fe", "Apple"),
    ("b0:8b:92", "Apple"),
    ("b0:8d:af", "Apple"),
    ("b0:9f:ba", "Apple"),
    ("b0:ac:92", "Apple"),
    ("b0:ca:e3", "Apple"),
    ("b0:d8:ae", "Apple"),
    ("b0:e8:c0", "Apple"),
    ("b0:eb:57", "Apple"),
    ("b0:f0:0c", "Apple"),
    ("b4:18:d1", "Apple"),
    ("b4:31:b6", "Apple"),
    ("b4:36:8b", "Apple"),
    ("b4:56:e9", "Apple"),
    ("b4:5f:be", "Apple"),
    ("b4:62:1f", "Apple"),
    ("b4:6d:c6", "Apple"),
    ("b4:7c:9c", "Apple"),
    ("b4:7f:5e", "Apple"),
    ("b4:8b:19", "Apple"),
    ("b4:98:76", "Apple"),
    ("b4:99:ba", "Apple"),
    ("b4:9c:df", "Apple"),
    ("b4:a3:86", "Apple"),
    ("b4:b5:af", "Apple"),
    ("b4:b6:86", "Apple"),
    ("b4:c4:fc", "Apple"),
    ("b4:ce:18", "Apple"),
    ("b4:d7:5c", "Apple"),
    ("b4:e5:5e", "Apple"),
    ("b4:e8:2d", "Apple"),
    ("b4:f0:db", "Apple"),
    ("b4:f6:1b", "Apple"),
    ("b4:fc:38", "Apple"),
    ("b8:09:8a", "Apple"),
    ("b8:17:c2", "Apple"),
    ("b8:17:e3", "Apple"),
    ("b8:27:eb", "Raspberry Pi"),
    ("b8:31:b5", "Apple"),
    ("b8:3d:4e", "Apple"),
    ("b8:41:5f", "Apple"),
    ("b8:44:af", "Apple"),
    ("b8:53:ac", "Apple"),
    ("b8:5d:0a", "Apple"),
    ("b8:66:85", "Apple"),
    ("b8:6b:23", "Apple"),
    ("b8:78:2e", "Apple"),
    ("b8:78:79", "Apple"),
    ("b8:81:98", "Apple"),
    ("b8:87:2e", "Apple"),
    ("b8:8d:12", "Apple"),
    ("b8:9a:ed", "Apple"),
    ("b8:9b:bc", "Apple"),
    ("b8:a5:a0", "Apple"),
    ("b8:ac:6f", "Apple"),
    ("b8:b1:c7", "Apple"),
    ("b8:c2:b5", "Apple"),
    ("b8:c7:5e", "Apple"),
    ("b8:c7:93", "Apple"),
    ("b8:d1:99", "Apple"),
    ("b8:d7:af", "Apple"),
    ("b8:e8:56", "Apple"),
    ("b8:e9:37", "Apple"),
    ("b8:ee:65", "Apple"),
    ("b8:f6:b1", "Apple"),
    ("b8:fa:74", "Apple"),
    ("bc:08:73", "Apple"),
    ("bc:0f:2b", "Apple"),
    ("bc:14:ef", "Apple"),
    ("bc:1a:8a", "Apple"),
    ("bc:25:e0", "Apple"),
    ("bc:2b:44", "Apple"),
    ("bc:2c:55", "Apple"),
    ("bc:3b:af", "Apple"),
    ("bc:4c:c4", "Apple"),
    ("bc:4e:3b", "Apple"),
    ("bc:52:b7", "Apple"),
    ("bc:57:1c", "Apple"),
    ("bc:5a:b0", "Apple"),
    ("bc:5f:f4", "Apple"),
    ("bc:6c:6e", "Apple"),
    ("bc:76:70", "Apple"),
    ("bc:77:37", "Apple"),
    ("bc:7a:8d", "Apple"),
    ("bc:7f:1d", "Apple"),
    ("bc:85:1e", "Apple"),
    ("bc:92:6b", "Apple"),
    ("bc:9f:ef", "Apple"),
    ("bc:aa:07", "Apple"),
    ("bc:a4:e1", "Apple"),
    ("bc:c0:ff", "Apple"),
    ("bc:c7:da", "Apple"),
    ("bc:d1:d3", "Apple"),
    ("bc:f2:af", "Apple"),
    ("c0:63:94", "Apple"),
    ("c0:84:7a", "Apple"),
    ("c0:9f:42", "Apple"),
    ("c0:a0:bb", "Apple"),
    ("c0:b5:81", "Apple"),
    ("c0:cc:6a", "Apple"),
    ("c0:ce:cd", "Apple"),
    ("c0:dc:36", "Apple"),
    ("c0:e4:22", "Apple"),
    ("c0:f2:fb", "Apple"),
    ("c4:14:11", "Apple"),
    ("c4:22:3f", "Apple"),
    ("c4:34:6b", "Apple"),
    ("c4:3d:c7", "Apple"),
    ("c4:47:3f", "Apple"),
    ("c4:64:13", "Apple"),
    ("c4:82:3f", "Apple"),
    ("c4:8e:5f", "Apple"),
    ("c4:98:80", "Apple"),
    ("c4:9e:43", "Apple"),
    ("c4:a3:66", "Apple"),
    ("c4:a7:2b", "Apple"),
    ("c4:b3:01", "Apple"),
    ("c4:b3:6c", "Apple"),
    ("c4:d5:8f", "Apple"),
    ("c4:e9:5f", "Apple"),
    ("c4:ee:22", "Apple"),
    ("c4:f3:12", "Apple"),
    ("c4:ff:1f", "Apple"),
    ("c8:15:45", "Apple"),
    ("c8:2a:14", "Apple"),
    ("c8:51:7f", "Apple"),
    ("c8:5e:6b", "Apple"),
    ("c8:69:cd", "Apple"),
    ("c8:84:39", "Apple"),
    ("c8:88:fa", "Apple"),
    ("c8:89:56", "Apple"),
    ("c8:8c:8f", "Apple"),
    ("c8:8f:a1", "Apple"),
    ("c8:93:46", "Apple"),
    ("c8:97:9b", "Apple"),
    ("c8:9e:43", "Apple"),
    ("c8:9f:d2", "Apple"),
    ("c8:a0:30", "Apple"),
    ("c8:a8:3d", "Apple"),
    ("c8:a8:de", "Apple"),
    ("c8:b2:e0", "Apple"),
    ("c8:b5:ad", "Apple"),
    ("c8:b5:e9", "Apple"),
    ("c8:b6:c1", "Apple"),
    ("c8:bc:4b", "Apple"),
    ("c8:bc:c8", "Apple"),
    ("c8:bc:e5", "Apple"),
    ("c8:c7:50", "Apple"),
    ("c8:d0:83", "Apple"),
    ("c8:d7:19", "Apple"),
    ("c8:e0:eb", "Apple"),
    ("c8:f6:50", "Apple"),
    ("c8:fc:21", "Apple"),
    ("cc:05:08", "Apple"),
    ("cc:08:8d", "Apple"),
    ("cc:0c:7f", "Apple"),
    ("cc:20:3f", "Apple"),
    ("cc:25:7f", "Apple"),
    ("cc:29:f5", "Apple"),
    ("cc:44:63", "Apple"),
    ("cc:44:b8", "Apple"),
    ("cc:4f:5c", "Apple"),
    ("cc:78:5f", "Apple"),
    ("cc:96:a0", "Apple"),
    ("cc:9f:7a", "Apple"),
    ("cc:a7:c0", "Apple"),
    ("cc:b0:da", "Apple"),
    ("cc:b0:e6", "Apple"),
    ("cc:b8:26", "Apple"),
    ("cc:c0:79", "Apple"),
    ("cc:c7:60", "Apple"),
    ("cc:e0:41", "Apple"),
    ("cc:f3:a5", "Apple"),
    ("cc:f9:57", "Apple"),
    ("d0:03:4b", "Apple"),
    ("d0:4b:ae", "Apple"),
    ("d0:63:b4", "Apple"),
    ("d0:81:7a", "Apple"),
    ("d0:90:52", "Apple"),
    ("d0:94:66", "Apple"),
    ("d0:a6:7e", "Apple"),
    ("d0:b0:76", "Apple"),
    ("d0:c5:f3", "Apple"),
    ("d0:cf:5c", "Apple"),
    ("d0:d2:b0", "Apple"),
    ("d0:e1:40", "Apple"),
    ("d0:e7:82", "Apple"),
    ("d0:fb:6c", "Apple"),
    ("d4:25:8b", "Apple"),
    ("d4:3b:04", "Apple"),
    ("d4:4f:42", "Apple"),
    ("d4:57:cf", "Apple"),
    ("d4:61:fe", "Apple"),
    ("d4:61:9e", "Apple"),
    ("d4:63:c6", "Apple"),
    ("d4:74:1b", "Apple"),
    ("d4:78:52", "Apple"),
    ("d4:7a:5f", "Apple"),
    ("d4:81:d6", "Apple"),
    ("d4:90:9a", "Apple"),
    ("d4:95:24", "Apple"),
    ("d4:97:0b", "Apple"),
    ("d4:9c:28", "Apple"),
    ("d4:a9:28", "Apple"),
    ("d4:b1:46", "Apple"),
    ("d4:b8:ff", "Apple"),
    ("d4:c6:7a", "Apple"),
    ("d4:d1:ad", "Apple"),
    ("d4:dc:cd", "Apple"),
    ("d4:e6:b7", "Apple"),
    ("d4:eb:68", "Apple"),
    ("d4:fc:13", "Apple"),
    ("d8:00:4d", "Apple"),
    ("d8:30:62", "Apple"),
    ("d8:3a:f3", "Apple"),
    ("d8:3b:bf", "Apple"),
    ("d8:3f:0c", "Apple"),
    ("d8:53:83", "Apple"),
    ("d8:54:3a", "Apple"),
    ("d8:58:e7", "Apple"),
    ("d8:5e:e8", "Apple"),
    ("d8:61:0d", "Apple"),
    ("d8:63:75", "Apple"),
    ("d8:68:c3", "Apple"),
    ("d8:6c:3a", "Apple"),
    ("d8:6c:af", "Apple"),
    ("d8:90:e8", "Apple"),
    ("d8:96:95", "Apple"),
    ("d8:9d:c1", "Apple"),
    ("d8:9e:3f", "Apple"),
    ("d8:9e:f9", "Apple"),
    ("d8:a2:5e", "Apple"),
    ("d8:b1:2e", "Apple"),
    ("d8:b4:04", "Apple"),
    ("d8:bb:2c", "Apple"),
    ("d8:bc:38", "Apple"),
    ("d8:c0:63", "Apple"),
    ("d8:c0:92", "Apple"),
    ("d8:d1:cb", "Apple"),
    ("d8:ef:f7", "Apple"),
    ("d8:f1:5b", "Apple"),
    ("dc:08:0f", "Apple"),
    ("dc:08:39", "Apple"),
    ("dc:2b:2a", "Apple"),
    ("dc:2b:61", "Apple"),
    ("dc:37:14", "Apple"),
    ("dc:4d:ac", "Apple"),
    ("dc:6c:5a", "Apple"),
    ("dc:74:a8", "Apple"),
    ("dc:86:d8", "Apple"),
    ("dc:8b:38", "Apple"),
    ("dc:9b:9c", "Apple"),
    ("dc:a4:ca", "Apple"),
    ("dc:b3:94", "Apple"),
    ("dc:d7:43", "Apple"),
    ("dc:e9:1c", "Apple"),
    ("dc:ec:5e", "Apple"),
    ("e0:18:77", "Apple"),
    ("e0:36:76", "Apple"),
    ("e0:45:c8", "Apple"),
    ("e0:5f:45", "Apple"),
    ("e0:88:5d", "Apple"),
    ("e0:8e:3c", "Apple"),
    ("e0:90:7e", "Apple"),
    ("e0:9d:31", "Apple"),
    ("e0:a8:c8", "Apple"),
    ("e0:b5:35", "Apple"),
    ("e0:b9:a5", "Apple"),
    ("e0:b9:ba", "Apple"),
    ("e0:c9:7a", "Apple"),
    ("e0:c9:a6", "Apple"),
    ("e0:c9:bc", "Apple"),
    ("e0:c9:d1", "Apple"),
    ("e0:d0:83", "Apple"),
    ("e0:d4:3b", "Apple"),
    ("e0:d7:ba", "Apple"),
    ("e0:d8:48", "Apple"),
    ("e0:f5:c6", "Apple"),
    ("e0:f8:47", "Apple"),
    ("e0:f8:78", "Apple"),
    ("e0:ff:fb", "Apple"),
    ("e4:02:c7", "Apple"),
    ("e4:02:9b", "Apple"),
    ("e4:0e:ee", "Apple"),
    ("e4:25:61", "Apple"),
    ("e4:5c:24", "Apple"),
    ("e4:64:cb", "Apple"),
    ("e4:79:c1", "Apple"),
    ("e4:98:d6", "Apple"),
    ("e4:9a:79", "Apple"),
    ("e4:ab:89", "Apple"),
    ("e4:b5:2b", "Apple"),
    ("e4:b6:17", "Apple"),
    ("e4:c6:3d", "Apple"),
    ("e4:c6:63", "Apple"),
    ("e4:c7:cb", "Apple"),
    ("e4:c7:dd", "Apple"),
    ("e4:d9:68", "Apple"),
    ("e4:e0:a6", "Apple"),
    ("e4:e1:2e", "Apple"),
    ("e4:e7:49", "Apple"),
    ("e4:fc:82", "Apple"),
    ("e4:fe:2f", "Apple"),
    ("e4:ff:dd", "Apple"),
    ("e8:04:0b", "Apple"),
    ("e8:04:95", "Apple"),
    ("e8:07:bf", "Apple"),
    ("e8:27:74", "Apple"),
    ("e8:2b:33", "Apple"),
    ("e8:3b:9f", "Apple"),
    ("e8:46:a6", "Apple"),
    ("e8:4e:06", "Apple"),
    ("e8:50:1b", "Apple"),
    ("e8:5b:e7", "Apple"),
    ("e8:65:8c", "Apple"),
    ("e8:6d:c4", "Apple"),
    ("e8:71:2b", "Apple"),
    ("e8:88:92", "Apple"),
    ("e8:8d:28", "Apple"),
    ("e8:92:a4", "Apple"),
    ("e8:94:35", "Apple"),
    ("e8:9a:8f", "Apple"),
    ("e8:a8:ae", "Apple"),
    ("e8:b4:c8", "Apple"),
    ("e8:c3:8a", "Apple"),
    ("e8:c7:4f", "Apple"),
    ("e8:e7:25", "Apple"),
    ("e8:ec:6e", "Apple"),
    ("e8:f2:e2", "Apple"),
    ("e8:fc:af", "Apple"),
    ("ec:10:7b", "Apple"),
    ("ec:12:34", "Apple"),
    ("ec:26:86", "Apple"),
    ("ec:35:86", "Apple"),
    ("ec:35:9a", "Apple"),
    ("ec:44:12", "Apple"),
    ("ec:44:76", "Apple"),
    ("ec:4d:48", "Apple"),
    ("ec:5a:86", "Apple"),
    ("ec:5a:92", "Apple"),
    ("ec:5f:66", "Apple"),
    ("ec:60:94", "Apple"),
    ("ec:66:1c", "Apple"),
    ("ec:66:c6", "Apple"),
    ("ec:7c:6c", "Apple"),
    ("ec:8e:b5", "Apple"),
    ("ec:95:e2", "Apple"),
    ("ec:9b:f0", "Apple"),
    ("ec:ad:b8", "Apple"),
    ("ec:c0:74", "Apple"),
    ("ec:d0:37", "Apple"),
    ("ec:d7:0c", "Apple"),
    ("ec:e0:9b", "Apple"),
    ("ec:e2:fd", "Apple"),
    ("f0:24:05", "Apple"),
    ("f0:37:17", "Apple"),
    ("f0:3f:95", "Apple"),
    ("f0:42:1c", "Apple"),
    ("f0:4d:a2", "Apple"),
    ("f0:52:25", "Apple"),
    ("f0:65:dd", "Apple"),
    ("f0:7b:cb", "Apple"),
    ("f0:8a:28", "Apple"),
    ("f0:93:c7", "Apple"),
    ("f0:99:bf", "Apple"),
    ("f0:9f:c2", "Apple"),
    ("f0:a4:79", "Apple"),
    ("f0:b0:52", "Apple"),
    ("f0:b4:79", "Apple"),
    ("f0:b6:eb", "Apple"),
    ("f0:b7:51", "Apple"),
    ("f0:c8:48", "Apple"),
    ("f0:c9:5c", "Apple"),
    ("f0:cb:a4", "Apple"),
    ("f0:ce:3e", "Apple"),
    ("f0:cf:df", "Apple"),
    ("f0:d1:a9", "Apple"),
    ("f0:db:e2", "Apple"),
    ("f0:dc:e2", "Apple"),
    ("f0:df:02", "Apple"),
    ("f0:e3:1c", "Apple"),
    ("f0:e3:46", "Apple"),
    ("f0:e4:e2", "Apple"),
    ("f0:e7:72", "Apple"),
    ("f0:e8:95", "Apple"),
    ("f0:e9:03", "Apple"),
    ("f0:ee:bb", "Apple"),
    ("f0:f1:af", "Apple"),
    ("f0:f4:6f", "Apple"),
    ("f0:f4:b3", "Apple"),
    ("f0:f5:ba", "Apple"),
    ("f0:f6:c2", "Apple"),
    ("f0:f9:47", "Apple"),
    ("f4:0f:24", "Apple"),
    ("f4:31:6c", "Apple"),
    ("f4:5c:89", "Apple"),
    ("f4:69:42", "Apple"),
    ("f4:5c:89", "Apple"),
    ("f4:7b:5e", "Apple"),
    ("f4:7e:4f", "Apple"),
    ("f4:82:6d", "Apple"),
    ("f4:93:c2", "Apple"),
    ("f4:98:6a", "Apple"),
    ("f4:9f:ff", "Apple"),
    ("f4:b1:5d", "Apple"),
    ("f4:b3:b2", "Apple"),
    ("f4:b7:e2", "Apple"),
    ("f4:d4:88", "Apple"),
    ("f4:f1:5a", "Apple"),
    ("f4:f5:a5", "Apple"),
    ("f4:f5:c4", "Apple"),
    ("f8:27:93", "Apple"),
    ("f8:27:2e", "Apple"),
    ("f8:27:93", "Apple"),
    ("f8:2c:18", "Apple"),
    ("f8:27:93", "Apple"),
    ("f8:27:93", "Apple"),
    ("f8:34:41", "Apple"),
    ("f8:4d:89", "Apple"),
    ("f8:50:8a", "Apple"),
    ("f8:58:8a", "Apple"),
    ("f8:60:e3", "Apple"),
    ("f8:67:2a", "Apple"),
    ("f8:87:f1", "Apple"),
    ("f8:89:b4", "Apple"),
    ("f8:98:b9", "Apple"),
    ("f8:9f:2a", "Apple"),
    ("f8:a9:d0", "Apple"),
    ("f8:ad:cb", "Apple"),
    ("f8:ae:9c", "Apple"),
    ("f8:bc:12", "Apple"),
    ("f8:bf:09", "Apple"),
    ("f8:c1:16", "Apple"),
    ("f8:c3:9e", "Apple"),
    ("f8:d4:76", "Apple"),
    ("f8:da:0c", "Apple"),
    ("f8:dc:7a", "Apple"),
    ("f8:e0:79", "Apple"),
    ("f8:e6:38", "Apple"),
    ("f8:e9:4e", "Apple"),
    ("f8:f1:e6", "Apple"),
    ("fc:18:3c", "Apple"),
    ("fc:25:3f", "Apple"),
    ("fc:2a:54", "Apple"),
    ("fc:2f:ef", "Apple"),
    ("fc:3f:db", "Apple"),
    ("fc:42:65", "Apple"),
    ("fc:44:32", "Apple"),
    ("fc:4c:a1", "Apple"),
    ("fc:4c:e7", "Apple"),
    ("fc:5c:ee", "Apple"),
    ("fc:65:de", "Apple"),
    ("fc:67:81", "Apple"),
    ("fc:88:16", "Apple"),
    ("fc:89:22", "Apple"),
    ("fc:8f:c4", "Apple"),
    ("fc:94:e3", "Apple"),
    ("fc:95:48", "Apple"),
    ("fc:a1:83", "Apple"),
    ("fc:a6:cd", "Apple"),
    ("fc:c2:de", "Apple"),
    ("fc:c6:69", "Apple"),
    ("fc:cc:14", "Apple"),
    ("fc:e5:5c", "Apple"),
    ("fc:e9:98", "Apple"),
];

pub fn mac_to_vendor(mac: &str) -> Option<String> {
    let normalized = normalize_mac_prefix(mac);
    OUI_VENDORS
        .iter()
        .find(|(oui, _)| normalized.starts_with(oui))
        .map(|(_, vendor)| vendor.to_string())
}

fn normalize_mac_prefix(mac: &str) -> String {
    let cleaned: String = mac
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();

    if cleaned.len() >= 6 {
        format!("{}:{}:{}", &cleaned[0..2], &cleaned[2..4], &cleaned[4..6])
    } else {
        cleaned
    }
}

pub fn get_hostname(ip: &str) -> Option<String> {
    use std::net::ToSocketAddrs;

    let addr = format!("{}:0", ip);
    match addr.to_socket_addrs() {
        Ok(addrs) => {
            for a in addrs {
                if let Ok(ip) = a.ip().to_string().parse::<IpAddr>() {
                    log::debug!("Resolved IP: {:?}", ip);
                }
            }
            None
        }
        Err(e) => {
            log::debug!("Failed to resolve hostname for {}: {}", ip, e);
            None
        }
    }
}

pub fn is_valid_mac(mac: &str) -> bool {
    let cleaned: String = mac
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();

    if cleaned.len() != 12 {
        return false;
    }

    cleaned.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn parse_mac(mac: &str) -> Result<[u8; 6]> {
    if !is_valid_mac(mac) {
        return Err(NetworkError::InvalidMacAddress(mac.to_string()));
    }

    let cleaned: String = mac
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();

    let mut result = [0u8; 6];
    for i in 0..6 {
        let byte_str = &cleaned[i * 2..i * 2 + 2];
        result[i] = u8::from_str_radix(byte_str, 16)
            .map_err(|_| NetworkError::InvalidMacAddress(mac.to_string()))?;
    }

    Ok(result)
}

pub fn format_mac(mac: &[u8; 6]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

pub fn parse_ip(ip: &str) -> Result<IpAddr> {
    IpAddr::from_str(ip).map_err(|_| NetworkError::InvalidIpAddress(ip.to_string()))
}

pub fn check_admin_privileges() -> Result<()> {
    #[cfg(unix)]
    {
        use std::process::Command;
        let output = Command::new("id").arg("-u").output().map_err(|e| {
            NetworkError::PermissionDenied(format!("Failed to check user ID: {}", e))
        })?;

        let uid = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u32>()
            .map_err(|_| NetworkError::PermissionDenied("Failed to parse user ID".to_string()))?;

        if uid != 0 {
            return Err(NetworkError::PermissionDenied(
                "Administrator/root privileges required for raw socket operations".to_string(),
            ));
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("net")
            .args(["session"])
            .output()
            .map_err(|_| {
                NetworkError::PermissionDenied(
                    "Administrator privileges required for raw socket operations".to_string(),
                )
            })?;

        if !output.status.success() {
            return Err(NetworkError::PermissionDenied(
                "Administrator privileges required for raw socket operations".to_string(),
            ));
        }
    }

    Ok(())
}

pub fn get_interface_ip(interface_name: &str) -> Result<String> {
    let interfaces = pnet_datalink::interfaces();

    let interface = interfaces
        .into_iter()
        .find(|iface| iface.name == interface_name)
        .ok_or_else(|| NetworkError::InterfaceNotFound(interface_name.to_string()))?;

    let ip = interface
        .ips
        .iter()
        .find(|ip| ip.is_ipv4())
        .map(|ip| ip.ip().to_string())
        .ok_or_else(|| {
            NetworkError::InterfaceNotFound(format!(
                "No IPv4 address on interface {}",
                interface_name
            ))
        })?;

    Ok(ip)
}

pub fn get_interface_mac(interface_name: &str) -> Result<String> {
    let interfaces = pnet_datalink::interfaces();

    let interface = interfaces
        .into_iter()
        .find(|iface| iface.name == interface_name)
        .ok_or_else(|| NetworkError::InterfaceNotFound(interface_name.to_string()))?;

    let mac = interface.mac.ok_or_else(|| {
        NetworkError::MacAddressError(format!("No MAC address on interface {}", interface_name))
    })?;

    Ok(format_mac(&mac.octets()))
}

pub fn generate_network_range(network_prefix: &str, netmask: &str) -> Vec<String> {
    let prefix_parts: Vec<&str> = network_prefix.split('.').collect();
    let mask_parts: Vec<&str> = netmask.split('.').collect();

    if prefix_parts.len() != 3 || mask_parts.len() != 4 {
        return Vec::new();
    }

    let mask_octet: u8 = mask_parts[3].parse().unwrap_or(0);
    let host_bits = 8 - mask_octet.leading_ones() as u8;

    if host_bits == 0 || host_bits > 8 {
        return Vec::new();
    }

    let num_hosts = 1u32 << host_bits;
    let base = prefix_parts.join(".");

    (1..num_hosts - 1)
        .map(|i| format!("{}.{}", base, i))
        .collect()
}

pub fn flush_arp_cache() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("arp")
            .arg("-an")
            .output()
            .map_err(|e| NetworkError::IoError(e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(ip) = line.split('(').nth(1).and_then(|s| s.split(')').next()) {
                let _ = std::process::Command::new("arp").args(["-d", ip]).output();
            }
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("ip")
            .args(["-s", "neigh", "flush", "all"])
            .output()
            .map_err(|e| NetworkError::IoError(e))?;

        if !output.status.success() {
            return Err(NetworkError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to flush ARP cache",
            )));
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("arp")
            .args(["-d", "*"])
            .output()
            .map_err(|e| NetworkError::IoError(e))?;

        if !output.status.success() {
            return Err(NetworkError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to flush ARP cache",
            )));
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err(NetworkError::PlatformNotSupported(
            "ARP cache flush not supported".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_mac() {
        assert!(is_valid_mac("aa:bb:cc:dd:ee:ff"));
        assert!(is_valid_mac("AA:BB:CC:DD:EE:FF"));
        assert!(is_valid_mac("aa-bb-cc-dd-ee-ff"));
        assert!(is_valid_mac("aabbccddeeff"));
        assert!(is_valid_mac("aabb.ccdd.eeff"));
        assert!(!is_valid_mac("aa:bb:cc:dd:ee"));
        assert!(!is_valid_mac("aa:bb:cc:dd:ee:ff:gg"));
        assert!(!is_valid_mac("not-a-mac"));
    }

    #[test]
    fn test_parse_mac() {
        let result = parse_mac("aa:bb:cc:dd:ee:ff").unwrap();
        assert_eq!(result, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);

        let result = parse_mac("aabbccddeeff").unwrap();
        assert_eq!(result, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn test_format_mac() {
        let mac = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
        assert_eq!(format_mac(&mac), "aa:bb:cc:dd:ee:ff");
    }

    #[test]
    fn test_mac_to_vendor() {
        assert_eq!(
            mac_to_vendor("80:02:18:00:00:00"),
            Some("Apple".to_string())
        );
        assert_eq!(
            mac_to_vendor("00:50:56:00:00:00"),
            Some("VMware".to_string())
        );
        assert_eq!(mac_to_vendor("ff:ff:ff:00:00:00"), None);
    }

    #[test]
    fn test_generate_network_range() {
        let range = generate_network_range("192.168.1", "255.255.255.0");
        assert_eq!(range.len(), 254);
        assert_eq!(range[0], "192.168.1.1");
        assert_eq!(range[253], "192.168.1.254");
    }
}
