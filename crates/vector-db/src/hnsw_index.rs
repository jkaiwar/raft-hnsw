use std::{
    cmp::{Reverse, min},
    collections::BinaryHeap,
};

use rand::RngExt;

type PointId = usize;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct Dist(f32);

struct HnswIndex<S: SelectNeighbours<V>, V: VectorStore> {
    params: HnswParameters,

    store: V,
    layers: Vec<Vec<Vec<(Dist, PointId)>>>,
    selector: S,

    entry_point: Option<(PointId, usize)>,

    visited_list: VisitedList,
    candidates: BinaryHeap<Reverse<(Dist, PointId)>>,
    results: BinaryHeap<(Dist, PointId)>,
    rng: rand::rngs::StdRng,
}

struct HnswParameters {
    ef_construction: usize,
    m: usize,
    m_max: usize,
    m_max0: usize,
    ml: f64,
}

impl<S: SelectNeighbours<V>, V: VectorStore> HnswIndex<S, V> {
    pub fn new() -> Self {
        todo!()
    }

    pub fn insert(&mut self, q: Vec<f32>) -> Result<(), HnswError> {
        if q.len() != self.store.dim() {
            return Err(HnswError::DimensionMismatch {
                expected: self.store.dim(),
                actual: q.len(),
            });
        }
        let id = self.store.push(q);

        let l = {
            let u = 1.0 - self.rng.random::<f64>();
            (-u.ln() * self.params.ml).floor() as usize
        };

        for layer in &mut self.layers {
            layer.push(Vec::new());
        }
        for _ in self.layers.len()..=l {
            self.layers.push(vec![Vec::new(); id + 1]);
        }

        if let Some((e, height)) = self.entry_point {
            let mut ep = vec![e];

            for lc in (l + 1..=height).rev() {
                self.search_layer(id, &ep, 1, lc);
                let (_, new) = self.results.pop().expect("expected layer to return vector");
                ep = vec![new];
            }

            for lc in (0..=min(l, height)).rev() {
                self.search_layer(id, &ep, self.params.ef_construction, lc);

                let neighbours = self.selector.select(
                    id,
                    self.results.as_slice(),
                    self.params.m,
                    &self.store,
                    &self.layers[lc],
                );
                for &(d, e) in &neighbours {
                    self.layers[lc][e].push((d, id));
                    self.layers[lc][id].push((d, e));
                }

                let m_max = match lc {
                    0 => self.params.m_max0,
                    _ => self.params.m_max,
                };
                for &(_, e) in &neighbours {
                    
                    if self.layers[lc][e].len() > m_max {
                        self.layers[lc][e] = self.selector.select(
                            e,
                            &self.layers[lc][e],
                            m_max,
                            &self.store,
                            &self.layers[lc],
                        );
                    }
                }

                ep = self.results.iter().map(|&(_, e)| e).collect();
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

    fn search_layer(&mut self, q: PointId, ep: &[PointId], ef: usize, lc: usize) -> () {
        self.visited_list.reset(self.store.len());
        self.candidates.clear();
        self.results.clear();

        for &id in ep {
            self.visited_list.insert(id);
            let d = self.store.dist(q, id);
            self.candidates.push(Reverse((d, id)));
            self.results.push((d, id));
        }

        while let Some(Reverse((dc, c))) = self.candidates.pop() {
            let &(df, _) = self.results.peek().unwrap();
            if dc > df {
                break;
            }
            for &(_, e) in &self.layers[lc][c] {
                if self.visited_list.insert(e) {
                    let &(df, _) = self.results.peek().unwrap();
                    let de = self.store.dist(e, q);
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

    fn k_nn_search(&mut self, q: Vec<f32>, k: usize, ef: usize) -> Result<Vec<(Dist, PointId)>, HnswError> {
        if q.len() != self.store.dim() {
            return Err(HnswError::DimensionMismatch {
                expected: self.store.dim(),
                actual: q.len(),
            });
        }
        let Some((e, h)) = self.entry_point else {
            return Ok(Vec::new());
        };

        let id = self.store.push(q);

        let mut ep = vec![e];
        for lc in (1..=h).rev() {
            self.search_layer(id, &ep, 1, lc);
            let &(_, nearest) = self.results.peek().expect("expected layer to return vector");
            ep[0] = nearest;
        }
        self.search_layer(id, &ep, ef.max(k), 0);

        let mut nearest = self.results.as_slice().to_vec();
        nearest.sort_unstable();
        nearest.truncate(k);

        self.store.pop();
        Ok(nearest)
    }
}

struct VisitedList {
    marks: Vec<u16>,
    epoch: u16,
}

impl VisitedList {
    pub fn new() -> Self {
        Self {
            marks: Vec::new(),
            epoch: 0,
        }
    }

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
        if *mark == self.epoch {
            false
        } else {
            *mark = self.epoch;
            true
        }
    }
}

impl Dist {
    fn new(d: f32) -> Self {
        if d.is_nan() {
            panic!("tried to build a NaN distance")
        } else {
            Self(d)
        }
    }
}

impl Eq for Dist {}

impl Ord for Dist {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

trait SelectNeighbours<V: VectorStore> {
    fn select(
        &self,
        q: PointId,
        candidates: &[(Dist, PointId)],
        m: usize,
        store: &V,
        layer: &[Vec<(Dist, PointId)>],
    ) -> Vec<(Dist, PointId)>;
}

struct SelectNeighboursSimple;
impl<V: VectorStore> SelectNeighbours<V> for SelectNeighboursSimple {
    fn select(
        &self,
        _q: PointId,
        candidates: &[(Dist, PointId)],
        m: usize,
        _store: &V,
        _layer: &[Vec<(Dist, PointId)>],
    ) -> Vec<(Dist, PointId)> {
        let mut neighbours = candidates.to_vec();
        if neighbours.len() > m {
            neighbours.select_nth_unstable(m);
            neighbours.truncate(m);
        }
        neighbours
    }
}

struct SelectNeighboursHeuristic {
    extend_candidates: bool,
    keep_pruned_connections: bool,
}

impl<V: VectorStore> SelectNeighbours<V> for SelectNeighboursHeuristic {
    fn select(
        &self,
        q: PointId,
        candidates: &[(Dist, PointId)],
        m: usize,
        store: &V,
        layer: &[Vec<(Dist, PointId)>],
    ) -> Vec<(Dist, PointId)> {
        if self.extend_candidates {
            for &(d, c) in candidates {
                for &adj in &layer[c] {
                    
                }
            }
        }
        todo!()
    }
}

trait VectorStore {
    fn dim(&self) -> usize;
    fn len(&self) -> usize;
    fn push(&mut self, v: Vec<f32>) -> PointId;
    fn pop(&mut self) -> Option<Vec<f32>>;
    fn get(&self, i: PointId) -> Option<&Vec<f32>>;
    fn dist(&self, a: PointId, b: PointId) -> Dist;
}

trait Metric {
    fn dist(&self, _: &[f32], _: &[f32]) -> Dist;
}

/// Keeps every vector in memory; a vector's id is its position in `vectors`.
struct InMemoryStore<M: Metric> {
    dim: usize,
    vectors: Vec<Vec<f32>>,
    metric: M,
}

impl<M: Metric> InMemoryStore<M> {
    pub fn new(dim: usize, metric: M) -> Self {
        Self {
            dim,
            vectors: Vec::new(),
            metric,
        }
    }
}

impl<M: Metric> VectorStore for InMemoryStore<M> {
    fn dim(&self) -> usize {
        self.dim
    }

    fn len(&self) -> usize {
        self.vectors.len()
    }

    fn push(&mut self, v: Vec<f32>) -> PointId {
        let id = self.vectors.len();
        self.vectors.push(v);
        id
    }

    fn pop(&mut self) -> Option<Vec<f32>> {
        self.vectors.pop()
    }

    fn get(&self, id: PointId) -> Option<&Vec<f32>> {
        self.vectors.get(id)
    }

    fn dist(&self, a: PointId, b: PointId) -> Dist {
        self.metric.dist(&self.vectors[a], &self.vectors[b])
    }
}

#[derive(Debug)]
pub enum HnswError {
    DimensionMismatch { expected: usize, actual: usize },
}

impl std::fmt::Display for HnswError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HnswError::DimensionMismatch { expected, actual } => {
                write!(f, "dimension mismatch: expected {expected}, got {actual}")
            }
        }
    }
}

impl std::error::Error for HnswError {}
