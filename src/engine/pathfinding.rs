use super::geom::Point;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Clone, Copy, Debug)]
pub struct Exits {
    items: [(usize, f32); 8],
    len: usize
}

impl Default for Exits {
    fn default() -> Exits {
        Exits { items: [(0, 0.0); 8], len: 0 }
    }
}

impl Exits {
    pub fn new() -> Exits {
        Exits::default()
    }

    pub fn push(&mut self, exit: (usize, f32)) {
        assert!(self.len < self.items.len(), "a tile has at most 8 exits");
        self.items[self.len] = exit;
        self.len += 1;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> std::slice::Iter<'_, (usize, f32)> {
        self.items[..self.len].iter()
    }
}

impl<'a> IntoIterator for &'a Exits {
    type Item = &'a (usize, f32);
    type IntoIter = std::slice::Iter<'a, (usize, f32)>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub trait BaseMap {
    fn is_opaque(&self, _idx: usize) -> bool {
        false
    }

    fn get_available_exits(&self, _idx: usize) -> Exits {
        Exits::new()
    }

    fn get_pathing_distance(&self, _idx1: usize, _idx2: usize) -> f32 {
        1.0
    }
}

pub trait Algorithm2D: BaseMap {
    fn dimensions(&self) -> Point;

    fn point2d_to_index(&self, pt: Point) -> usize {
        (pt.y * self.dimensions().x + pt.x) as usize
    }

    fn index_to_point2d(&self, idx: usize) -> Point {
        let w = self.dimensions().x as usize;
        Point::new((idx % w) as i32, (idx / w) as i32)
    }

    fn in_bounds(&self, pos: Point) -> bool {
        let d = self.dimensions();
        pos.x >= 0 && pos.x < d.x && pos.y >= 0 && pos.y < d.y
    }
}

const MAX_ASTAR_STEPS: usize = 65_536;

pub struct NavigationPath {
    pub destination: usize,
    pub success: bool,
    pub steps: Vec<usize>
}

#[derive(Clone, Copy, PartialEq)]
struct Node {
    f: f32,
    idx: usize
}

impl Eq for Node {}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.total_cmp(&self.f).then_with(|| other.idx.cmp(&self.idx))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn a_star_search(start: usize, end: usize, map: &dyn BaseMap) -> NavigationPath {
    let mut result = NavigationPath { destination: end, success: false, steps: Vec::new() };
    if start == end {
        result.success = true;
        result.steps.push(start);
        return result;
    }

    let mut open = BinaryHeap::new();
    let mut g: HashMap<usize, f32> = HashMap::new();
    let mut parent: HashMap<usize, usize> = HashMap::new();
    g.insert(start, 0.0);
    open.push(Node { f: map.get_pathing_distance(start, end), idx: start });

    let mut expanded = 0;
    while let Some(Node { f, idx }) = open.pop() {
        if idx == end {
            let mut cur = end;
            result.steps.push(cur);
            while let Some(&p) = parent.get(&cur) {
                result.steps.push(p);
                cur = p;
            }
            result.steps.reverse();
            result.success = true;
            return result;
        }

        let g_here = g[&idx];
        if f > g_here + map.get_pathing_distance(idx, end) + 1e-4 {
            continue;
        }
        expanded += 1;
        if expanded > MAX_ASTAR_STEPS {
            break;
        }

        for &(next, cost) in map.get_available_exits(idx).iter() {
            let tentative = g_here + cost;
            if g.get(&next).is_none_or(|&old| tentative < old) {
                g.insert(next, tentative);
                parent.insert(next, idx);
                open.push(Node { f: tentative + map.get_pathing_distance(next, end), idx: next });
            }
        }
    }
    result
}

pub struct DijkstraMap {
    pub map: Vec<f32>,
    pub size_x: usize,
    pub size_y: usize,
    pub max_depth: f32,
}

impl DijkstraMap {
    pub fn new(size_x: usize, size_y: usize, starts: &[usize], map: &dyn BaseMap, max_depth: f32) -> DijkstraMap {
        let mut dm = DijkstraMap { map: vec![f32::MAX; size_x * size_y], size_x, size_y, max_depth };
        let mut open = BinaryHeap::new();
        for &s in starts {
            if s< dm.map.len() {
                dm.map[s] = 0.0;
                open.push(Node { f: 0.0, idx: s});
            }
        }

        while let Some(Node { f, idx}) = open.pop() {
            if f > dm.map[idx] {
                continue;
            }
            for &(next, cost) in map.get_available_exits(idx).iter() {
                let d = f + cost;
                if next < dm.map.len() && d <= max_depth && d < dm.map[next] {
                    dm.map[next] = d;
                    open.push(Node { f: d, idx: next });
                }
            }
        }
        dm
    }

    pub fn find_highest_exit(dm: &DijkstraMap, idx: usize, map: &dyn BaseMap) -> Option<usize> {
        map.get_available_exits(idx)
            .iter()
            .filter(|(e, _)| dm.map.get(*e).is_some_and(|&v| v < f32::MAX))
            .max_by(|a, b| dm.map[a.0].total_cmp(&dm.map[b.0]))
            .map(|e| e.0)
    }

    pub fn find_lowest_exit(dm: &DijkstraMap, idx: usize, map: &dyn BaseMap) -> Option<usize> {
        map.get_available_exits(idx)
            .iter()
            .filter(|(e, _)| dm.map.get(*e).is_some_and(|&v| v < f32::MAX))
            .min_by(|a, b| dm.map[a.0].total_cmp(&dm.map[b.0]))
            .map(|e| e.0)
    }
}

pub fn field_of_view(origin: Point, range: i32, map: &dyn Algorithm2D) -> Vec<Point> {
    let range = range.max(0);
    let side = (2 * range + 1) as usize;
    let mut seen = vec![false; side * side];
    let mut out = Vec::new();

    let mut reveal = |p: Point, out: &mut Vec<Point>| {
        let (dx, dy) = (p.x - origin.x, p.y - origin.y);
        if dx * dx + dy * dy > range * range || !map.in_bounds(p) {
            return;
        }
        let k = (dy + range) as usize * side + (dx + range) as usize;
        if !seen[k] {
            seen[k] = true;
            out.push(p);
        }
    };
    reveal(origin, &mut out);

    let opaque = |p: Point| !map.in_bounds(p) || map.is_opaque(map.point2d_to_index(p));
    for quadrant in 0..4 {
        let transform = move |depth: i32, col: i32| match quadrant {
             0 => Point::new(origin.x + col, origin.y - depth), // north
            1 => Point::new(origin.x + depth, origin.y + col), // east
            2 => Point::new(origin.x + col, origin.y + depth), // south
            _ => Point::new(origin.x - depth, origin.y + col), // west
        };
        let mut rows = vec![(1, -1.0f64, 1.0f64)];
        while let Some((depth, mut start, end)) = rows.pop() {
            if depth > range {
                continue;
            }
            let min_col = (depth as f64 * start + 0.5).floor() as i32;
            let max_col = (depth as f64 * end - 0.5).ceil() as i32;
            let mut prev_wall: Option<bool> = None;
            for col in min_col..=max_col {
                let p = transform(depth, col);
                let wall = opaque(p);
                let symmetric = col as f64 >= depth as f64 * start && col as f64 <= depth as f64 * end;
                if wall || symmetric {
                    reveal(p, &mut out)
                }
                let slope = (2 * col - 1) as f64 / (2 * depth) as f64;
                if prev_wall == Some(true) && !wall {
                    start = slope;
                }
                if prev_wall == Some(false) && wall {
                    rows.push((depth + 1, start, slope));
                }
                prev_wall = Some(wall)
            }
            if prev_wall == Some(false) {
                rows.push((depth + 1, start, end));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
 
    /// '#' = wall, '.' = floor.
    struct Grid {
        w: i32,
        h: i32,
        walls: Vec<bool>,
    }
 
    impl Grid {
        fn parse(rows: &[&str]) -> Grid {
            let walls = rows.iter().flat_map(|r| r.chars().map(|c| c == '#')).collect();
            Grid { w: rows[0].len() as i32, h: rows.len() as i32, walls }
        }
    }
 
    impl BaseMap for Grid {
        fn is_opaque(&self, idx: usize) -> bool {
            self.walls[idx]
        }
        fn get_available_exits(&self, idx: usize) -> Exits {
            let mut e = Exits::new();
            let p = self.index_to_point2d(idx);
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let n = Point::new(p.x + dx, p.y + dy);
                if self.in_bounds(n) && !self.walls[self.point2d_to_index(n)] {
                    e.push((self.point2d_to_index(n), 1.0));
                }
            }
            e
        }
        fn get_pathing_distance(&self, a: usize, b: usize) -> f32 {
            let (a, b) = (self.index_to_point2d(a), self.index_to_point2d(b));
            ((a.x - b.x).abs() + (a.y - b.y).abs()) as f32
        }
    }
 
    impl Algorithm2D for Grid {
        fn dimensions(&self) -> Point {
            Point::new(self.w, self.h)
        }
    }
 
    const MAZE: [&str; 5] = [
        "#########",
        "#...#...#",
        "#.#.#.#.#",
        "#.#...#.#",
        "#########",
    ];
 
    #[test]
    fn astar_finds_shortest_path_around_walls() {
        let g = Grid::parse(&MAZE);
        let (s, e) = (g.point2d_to_index(Point::new(1, 1)), g.point2d_to_index(Point::new(7, 3)));
        let path = a_star_search(s, e, &g);
        assert!(path.success);
        assert_eq!((path.steps[0], *path.steps.last().unwrap()), (s, e));
        assert_eq!(path.steps.len(), 13);
        for w in path.steps.windows(2) {
            assert_eq!(g.get_pathing_distance(w[0], w[1]), 1.0);
        }
        let blocked = Grid::parse(&["#####", "#.#.#", "#####"]);
        let p = a_star_search(blocked.point2d_to_index(Point::new(1, 1)), blocked.point2d_to_index(Point::new(3, 1)), &blocked);
        assert!(!p.success);
    }
 
    #[test]
    fn dijkstra_distances_and_exits() {
        let g = Grid::parse(&MAZE);
        let start = g.point2d_to_index(Point::new(1, 1));
        let dm = DijkstraMap::new(9, 5, &[start], &g, 100.0);
        assert_eq!(dm.map[start], 0.0);
        assert_eq!(dm.map[g.point2d_to_index(Point::new(7, 3))], 12.0);
        assert_eq!(dm.map[0], f32::MAX);
        let here = g.point2d_to_index(Point::new(1, 2));
        assert_eq!(DijkstraMap::find_highest_exit(&dm, here, &g), Some(g.point2d_to_index(Point::new(1, 3))));
        assert_eq!(DijkstraMap::find_lowest_exit(&dm, here, &g), Some(start));
    }
 
    #[test]
    fn fov_is_blocked_by_walls_and_symmetric() {
        let g = Grid::parse(&[
            "###########",
            "#.........#",
            "#....#....#",
            "#.........#",
            "###########",
        ]);
        let eye = Point::new(2, 2);
        let fov = field_of_view(eye, 10, &g);
        assert!(fov.contains(&eye));
        assert!(fov.contains(&Point::new(5, 2)), "the wall itself is visible");
        assert!(!fov.contains(&Point::new(7, 2)), "tiles straight behind the pillar are hidden");
        assert!(!fov.contains(&Point::new(9, 1)), "(9, 1)'s slope (-1/7) is inside the pillar's shadow (+-1/6)");
        for p in [(6, 1), (6, 3), (4, 1), (1, 3), (9, 0)] {
            assert!(fov.contains(&Point::new(p.0, p.1)), "{p:?} should be visible");
        }
        // Symmetry: if A sees B (both floor), B sees A.
        for p in fov.iter().filter(|p| !g.walls[g.point2d_to_index(**p)]) {
            assert!(field_of_view(*p, 10, &g).contains(&eye), "{p:?} is seen but can't see back");
        }
        let mut dedup = fov.clone();
        dedup.sort_by_key(|p| (p.x, p.y));
        dedup.dedup();
        assert_eq!(dedup.len(), fov.len());
        assert!(field_of_view(eye, 1, &g).iter().all(|p| (p.x - 2).pow(2) + (p.y - 2).pow(2) <= 1));
    }
}