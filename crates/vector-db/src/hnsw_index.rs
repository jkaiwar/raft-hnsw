use std::{cmp::Reverse, collections::BinaryHeap};

type PointId = usize;

#[derive(Debug, Clone, Copy, PartialEq , PartialOrd)]
struct Dist(f32);

struct HnswIndex {
    dim: usize,
    entry_point: Option<PointId>,
    vectors: Vec<Vec<f32>>,
    layers: Vec<Vec<PointId>>,
    params: HnswParameters,

    visited_list: VisitedList,
    candidates: BinaryHeap<Reverse<(Dist, PointId)>>,
    results: BinaryHeap<(Dist, PointId)>,
}

struct HnswParameters {
    ef_construction: bool,
    m: bool,
    m_max: bool,
    ml: bool,
}

impl HnswIndex {
    pub fn new() -> Self {
        todo!()
    }
    pub fn insert(&mut self, q : Vec<f32>) -> Result<(), HnswError> {
        if q.len() != self.dim {
            Err(HnswError::DimensionMismatch{expected: self.dim, actual: q.len()})
        } else {
            let id = self.vectors.len();
            self.vectors.push(q);

            
            todo!()
        }
    }

    fn search_layer(&mut self, q : PointId, ep: &[PointId], _ef: usize, _lc: usize) -> Vec<PointId> {
        self.visited_list.reset(self.vectors.len());
        self.candidates.clear();
        self.results.clear();
        
        for &id in ep {
            self.visited_list.insert(id);
            let d = self.dist( q, id);
            self.candidates.push(Reverse((d, id)));
            self.results.push((d,id));
        }

        while let Some(Reverse((dc, candidate))) = self.candidates.pop() {
            let &(df, f) = self.results.peek().unwrap();
            if dc < df {break}
        }

        todo!()
    }
    
    fn dist(&self, _x: PointId, _y: PointId) -> Dist {
        todo!()
    }
}


struct VisitedList {
    marks: Vec<u16>,
    epoch: u16
}

impl VisitedList {
    pub fn new() -> Self { Self { marks: Vec::new(), epoch: 0,} }
    
    pub fn reset(&mut self, n: usize) {
        if self.marks.len() < n {
            self.marks.resize(n, 0)
        }
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.marks.fill(0);
            self.epoch = 1
        }
    }

    pub fn insert(&mut self, q: PointId) -> bool {
        let mark = &mut self.marks[q];
        if *mark == self.epoch { false } else { *mark = self.epoch; true }
    }
}


impl Dist {
    fn new(d: f32) -> Self {
        if d.is_nan() { panic!("tried to build a NaN distance") } else { Self(d) }
    }
}

impl Eq for Dist {}

impl Ord for Dist {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}


#[derive(Debug)]
pub enum HnswError {
    DimensionMismatch {
        expected: usize,
        actual: usize,
    }
}

impl std::fmt::Display for HnswError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HnswError::DimensionMismatch { expected, actual } => {
                write! (f, "dimension mismatch: expected {expected}, got {actual}")
            }
        }
    }
}

impl std::error::Error for HnswError {}
