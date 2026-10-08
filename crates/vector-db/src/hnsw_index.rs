struct HNSW_Index {
    dim: usize,
    entry_point: bool,
    params: &HNSW_Parameters,
}

struct HNSW_Parameters {
    ef_construction: bool,
    m: bool,
    m_max: bool,
    ml: bool,
}

impl HNSW_Index {
    pub fn new() {
    }
    
    pub fn insert(&self) {
        (self.usize == 3)
    }
}

