use std::path::Path;
use std::sync::Arc;
use hdl_graph_core::*;
use rocksdb::{DB, ColumnFamilyDescriptor, Options, Direction, IteratorMode, WriteBatch, ColumnFamily};

pub struct RocksGraphStore {
    db: Arc<DB>,
}

impl RocksGraphStore {
    pub fn open(path: &Path) -> CoreResult<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_names = ["nodes", "edges_by_source", "edges_by_target", "node_properties", "symbol_table", "version_info"];
        let cfs: Vec<ColumnFamilyDescriptor> = cf_names.iter()
            .map(|name| ColumnFamilyDescriptor::new(*name, Options::default()))
            .collect();

        let db = DB::open_cf_descriptors(&opts, path, cfs)
            .map_err(|e| CoreError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        Ok(Self { db: Arc::new(db) })
    }

    fn u64_to_key(id: u64) -> [u8; 8] { id.to_be_bytes() }
    fn key_to_u64(key: &[u8]) -> u64 {
        let arr: [u8; 8] = key[..8].try_into().unwrap_or([0; 8]);
        u64::from_be_bytes(arr)
    }
    fn edge_key(source: u64, edge_type: EdgeType, target: u64) -> Vec<u8> {
        let mut key = Vec::with_capacity(17);
        key.extend_from_slice(&source.to_be_bytes());
        key.push(edge_type as u8);
        key.extend_from_slice(&target.to_be_bytes());
        key
    }

    fn edge_type_from_byte(b: u8) -> Option<EdgeType> {
        match b {
            0 => Some(EdgeType::Contains),
            1 => Some(EdgeType::Defines),
            2 => Some(EdgeType::References),
            3 => Some(EdgeType::Imports),
            4 => Some(EdgeType::Extends),
            5 => Some(EdgeType::Instantiates),
            6 => Some(EdgeType::Connects),
            7 => Some(EdgeType::Drives),
            8 => Some(EdgeType::Triggers),
            9 => Some(EdgeType::Calls),
            10 => Some(EdgeType::Overrides),
            11 => Some(EdgeType::MacroExpands),
            12 => Some(EdgeType::FactoryRegisters),
            13 => Some(EdgeType::FactoryOverrides),
            14 => Some(EdgeType::TLMBinds),
            15 => Some(EdgeType::ConfigSets),
            16 => Some(EdgeType::ConfigGets),
            17 => Some(EdgeType::ConfigResolves),
            _ => None,
        }
    }

    fn cf(&self, name: &str) -> &ColumnFamily {
        self.db.cf_handle(name).unwrap()
    }

    pub fn add_node_raw(&self, node: &GraphNode) -> CoreResult<()> {
        let key = Self::u64_to_key(node.id);
        let value = serde_json::to_vec(node)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;
        self.db.put_cf(&self.cf("nodes"), key, value)
            .map_err(|e| CoreError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
    }

    pub fn get_node_raw(&self, id: u64) -> CoreResult<Option<GraphNode>> {
        let key = Self::u64_to_key(id);
        match self.db.get_cf(&self.cf("nodes"), key) {
            Ok(Some(ref bytes)) => {
                let node: GraphNode = serde_json::from_slice(bytes)
                    .map_err(|e| CoreError::Serialization(e.to_string()))?;
                Ok(Some(node))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(CoreError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))),
        }
    }

    pub fn add_edge_raw(&self, edge: &Edge) -> CoreResult<()> {
        let sk = Self::edge_key(edge.source, edge.edge_type, edge.target);
        self.db.put_cf(&self.cf("edges_by_source"), sk, [0u8; 0])
            .map_err(|e| CoreError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        let tk = Self::edge_key(edge.target, edge.edge_type, edge.source);
        self.db.put_cf(&self.cf("edges_by_target"), tk, [0u8; 0])
            .map_err(|e| CoreError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        Ok(())
    }

    pub fn get_outgoing_raw(&self, node_id: u64) -> CoreResult<Vec<Edge>> {
        let prefix = Self::u64_to_key(node_id);
        let mut edges = Vec::new();
        let iter = self.db.iterator_cf(&self.cf("edges_by_source"), IteratorMode::From(&prefix, Direction::Forward));
        for item in iter {
            let (key, _) = item.map_err(|e| CoreError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            if key.len() < 17 || key[..8] != prefix { break; }
            if let Some(et) = Self::edge_type_from_byte(key[8]) {
                let target = Self::key_to_u64(&key[9..17]);
                edges.push(Edge { source: node_id, target, edge_type: et });
            }
        }
        Ok(edges)
    }

    pub fn get_incoming_raw(&self, node_id: u64) -> CoreResult<Vec<Edge>> {
        let prefix = Self::u64_to_key(node_id);
        let mut edges = Vec::new();
        let iter = self.db.iterator_cf(&self.cf("edges_by_target"), IteratorMode::From(&prefix, Direction::Forward));
        for item in iter {
            let (key, _) = item.map_err(|e| CoreError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            if key.len() < 17 || key[..8] != prefix { break; }
            if let Some(et) = Self::edge_type_from_byte(key[8]) {
                let source = Self::key_to_u64(&key[9..17]);
                edges.push(Edge { source, target: node_id, edge_type: et });
            }
        }
        Ok(edges)
    }

    pub fn batch_write(&self, nodes: &[GraphNode], edges: &[Edge]) -> CoreResult<()> {
        let mut batch = WriteBatch::default();
        for node in nodes {
            let key = Self::u64_to_key(node.id);
            let value = serde_json::to_vec(node)
                .map_err(|e| CoreError::Serialization(e.to_string()))?;
            batch.put_cf(&self.cf("nodes"), key, value);
        }
        for edge in edges {
            let sk = Self::edge_key(edge.source, edge.edge_type, edge.target);
            batch.put_cf(&self.cf("edges_by_source"), sk, [0u8; 0]);
            let tk = Self::edge_key(edge.target, edge.edge_type, edge.source);
            batch.put_cf(&self.cf("edges_by_target"), tk, [0u8; 0]);
        }
        self.db.write(batch)
            .map_err(|e| CoreError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        Ok(())
    }

    pub fn node_count_raw(&self) -> usize {
        self.db.iterator_cf(&self.cf("nodes"), IteratorMode::Start).count()
    }

    pub fn edge_count_raw(&self) -> usize {
        self.db.iterator_cf(&self.cf("edges_by_source"), IteratorMode::Start).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_open() {
        let dir = TempDir::new().unwrap();
        let store = RocksGraphStore::open(dir.path());
        assert!(store.is_ok());
    }

    #[test]
    fn test_add_and_get_node() {
        let dir = TempDir::new().unwrap();
        let store = RocksGraphStore::open(dir.path()).unwrap();
        let node = GraphNode { id: 42, kind: NodeKind::SourceFile, ..Default::default() };
        store.add_node_raw(&node).unwrap();
        let fetched = store.get_node_raw(42).unwrap().unwrap();
        assert_eq!(fetched.id, 42);
        assert!(matches!(fetched.kind, NodeKind::SourceFile));
    }

    #[test]
    fn test_edge_read_write() {
        let dir = TempDir::new().unwrap();
        let store = RocksGraphStore::open(dir.path()).unwrap();
        let a = GraphNode { id: 1, kind: NodeKind::SourceFile, ..Default::default() };
        let b = GraphNode { id: 2, kind: NodeKind::Module { name: InternedString(10) }, ..Default::default() };
        store.add_node_raw(&a).unwrap();
        store.add_node_raw(&b).unwrap();
        store.add_edge_raw(&Edge { source: 1, target: 2, edge_type: EdgeType::Contains }).unwrap();

        let out = store.get_outgoing_raw(1).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].target, 2);
        assert_eq!(out[0].edge_type as u8, EdgeType::Contains as u8);
    }

    #[test]
    fn test_batch_write() {
        let dir = TempDir::new().unwrap();
        let store = RocksGraphStore::open(dir.path()).unwrap();
        store.batch_write(
            &[
                GraphNode { id: 1, kind: NodeKind::Module { name: InternedString(1) }, ..Default::default() },
                GraphNode { id: 2, kind: NodeKind::Module { name: InternedString(2) }, ..Default::default() },
            ],
            &[Edge { source: 1, target: 2, edge_type: EdgeType::Contains }],
        ).unwrap();
        assert_eq!(store.node_count_raw(), 2);
        assert!(store.get_node_raw(1).unwrap().is_some());
        assert!(store.get_node_raw(2).unwrap().is_some());
    }
}
