use std::{cmp::{Reverse, min}, collections::BinaryHeap};

use rand::RngExt;

type PointId = usize;

#[derive(Debug, Clone, Copy, PartialEq , PartialOrd)]
struct Dist(f32);

struct HnswIndex {
    dim: usize,
    entry_point: Option<(PointId, usize)>,
    vectors: Vec<Vec<f32>>,
    layers: Vec<Vec<BinaryHeap<(Dist, PointId)>>>,
    params: HnswParameters,

    visited_list: VisitedList,
    candidates: BinaryHeap<Reverse<(Dist, PointId)>>,
    results: BinaryHeap<(Dist, PointId)>,
    rng: rand::rngs::StdRng,
}

struct HnswParameters {
    ef_construction: usize,
    m: usize,
    m_max: usize,
    ml: f64,
}

impl HnswIndex {
    pub fn new() -> Self {
        todo!()
    }
    pub fn insert(&mut self, q : Vec<f32>) -> Result<(), HnswError> {
        if q.len() != self.dim {
            return Err(HnswError::DimensionMismatch{expected: self.dim, actual: q.len()})
        }
        let id = self.vectors.len();
        self.vectors.push(q);
        
        let l = {
            let u = 1.0 - self.rng.random::<f64>();
            (- u.ln() * self.params.ml).floor() as usize
        };
        
        for layer in &mut self.layers {
            layer.push(BinaryHeap::new());
        }
        for _ in self.layers.len()..=l {
            self.layers.push(vec![BinaryHeap::new(); id + 1]);
        }
        
        if let Some((e, height)) = self.entry_point {
            let mut ep = vec!(e);
            
            // mutate ep to be the nearest point in layers above l to q.
            for lc in (l + 1..=height).rev() {
                self.search_layer(id, &ep, 1, lc);
                let (_, new) = self.results
                    .pop()
                    .expect("expected layer to return vector"); 
                ep = vec!(new);

            }
            
            // insert bidi connections in lower levels to efconstruction nearest elements and shrink connections.
            for lc in (0..=min(l, height)).rev() {
                self.search_layer(id, &ep, self.params.ef_construction, lc);
                ep = self.results.iter().map(|&(_, e)| e).collect();
                
                let neighbours = self.select_neighbors(self.params.m);
                for &(d, e) in &neighbours {
                    self.layers[lc][e].push((d,id));
                    self.layers[lc][id].push((d,e));
                }
                for &(_, e) in &neighbours {
                    while self.layers[lc][e].len() > self.params.m_max {
                        self.layers[lc][e].pop();
                    };
                }
            }
            
            // update the index entrypoint if l is higher than the height before insert.
            if l > height {
                self.entry_point = Some((id, l));
            }
            Ok(())
        } else {
            self.entry_point = Some((id, l));
            Ok(())
        }
    }

    fn search_layer(&mut self, q : PointId, ep: &[PointId], ef: usize, lc: usize) -> () {
        self.visited_list.reset(self.vectors.len());
        self.candidates.clear();
        self.results.clear();
        
        for &id in ep {
            self.visited_list.insert(id);
            let d = self.dist( q, id);
            self.candidates.push(Reverse((d, id)));
            self.results.push((d,id));
        }

        while let Some(Reverse((dc, c))) = self.candidates.pop() {
            let &(df, _) = self.results.peek().unwrap();
            if dc >= df {
                for &(_, e) in &self.layers[lc][c] {
                    if self.visited_list.insert(e) {
                        let &(df, _) = self.results.peek().unwrap();
                        let de = self.dist(e, q);
                        if de < df || self.results.len() < ef {
                            self.candidates.push(Reverse((de, e)));
                            self.results.push((de, e));
                            if self.results.len() > ef {
                                self.results.pop().unwrap();
                            }
                        }
                    }
                }
            }
        }
    }

    fn select_neighbors(&mut self, m: usize) -> Vec<(Dist, PointId)> {
        while self.results.len() > m {
            self.results.pop();
        };
        self.results.iter().copied().collect()
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
