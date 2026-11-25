use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Point {
        Point { x, y }
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, o: Point) -> Point {
        Point::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Point {
    type Output = Point;
    fn sub(self, o: Point) -> Point {
        Point::new(self.x - o.x, self.y - o.y)
    }
}

impl From<(i32, i32)> for Point {
    fn from((x, y): (i32, i32)) -> Point {
        Point::new(x, y)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointF {
    pub x: f32,
    pub y: f32,
}

impl PointF {
    pub const fn new(x: f32, y: f32) -> PointF {
        PointF { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    #[default]
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Direction {
    pub const ALL: [Direction; 8] = [
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ];

    fn index(self) -> i32 {
        Direction::ALL.iter().position(|&d| d == self).unwrap() as i32
    }

    pub fn delta(self) -> (i32, i32) {
        match self {
            Direction::North => (0, -1),
            Direction::NorthEast => (1, -1),
            Direction::East => (1, 0),
            Direction::SouthEast => (1, 1),
            Direction::South => (0, 1),
            Direction::SouthWest => (-1, 1),
            Direction::West => (-1, 0),
            Direction::NorthWest => (-1, -1),
        }
    }

    pub fn from_delta(dx: i32, dy: i32) -> Option<Direction> {
        let d = (dx.signum(), dy.signum());
        Direction::ALL.into_iter().find(|dir| dir.delta() == d)
    }

    pub fn rotated(self, eighths: i32) -> Direction {
        Direction::ALL[(self.index() + eighths).rem_euclid(8) as usize]
    }

    pub fn is_cardinal(self) -> bool {
        self.index() % 2 == 0
    }

    pub fn angle(self) -> f32 {
        (self.index() - 2) as f32 * std::f32::consts::FRAC_PI_4
    }

    pub fn name(self) -> &'static str {
        match self {
            Direction::North => "North",
            Direction::NorthEast => "North-east",
            Direction::East => "East",
            Direction::SouthEast => "South-east",
            Direction::South => "South",
            Direction::SouthWest => "South-west",
            Direction::West => "West",
            Direction::NorthWest => "North-west",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistanceAlg {
    Pythagoras,
    PythagorasSquared,
    Manhattan,
    Chebyshev,
}

impl DistanceAlg {
    pub fn distance2d(self, a: Point, b: Point) -> f32 {
        let dx = (a.x - b.x).abs() as f32;
        let dy = (a.y - b.y).abs() as f32;
        match self {
            DistanceAlg::Pythagoras => (dx * dx + dy * dy).sqrt(),
            DistanceAlg::PythagorasSquared => dx * dx + dy * dy,
            DistanceAlg::Manhattan => dx + dy,
            DistanceAlg::Chebyshev => dx.max(dy),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineAlg {
    Bresenham,
}

pub fn line2d(_alg: LineAlg, start: Point, end: Point) -> Vec<Point> {
    let (dx, dy) = ((end.x - start.x).abs(), -(end.y - start.y).abs());
    let (sx, sy) = ((end.x - start.x).signum(), (end.y - start.y).signum());
    let mut err = dx + dy;
    let mut p = start;
    let mut out = Vec::with_capacity((dx.max(-dy) + 1) as usize);
    loop {
        out.push(p);
        if p == end {
            return out;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            p.x += sx;
        }
        if e2 <= dx {
            err += dx;
            p.y += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_covers_both_ends() {
        let l = line2d(LineAlg::Bresenham, Point::new(0, 0), Point::new(4, 2));
        assert_eq!(l.first(), Some(&Point::new(0, 0)));
        assert_eq!(l.last(), Some(&Point::new(4, 2)));
        assert_eq!(l.len(), 5);
        assert_eq!(
            line2d(LineAlg::Bresenham, Point::new(3, 3), Point::new(3, 3)),
            vec![Point::new(3, 3)]
        );
    }

    #[test]
    fn directions() {
        assert_eq!(Direction::North.rotated(2), Direction::East);
        assert_eq!(Direction::North.rotated(-2), Direction::West);
        assert_eq!(Direction::West.rotated(9), Direction::NorthWest);
        assert_eq!(Direction::from_delta(5, -3), Some(Direction::NorthEast));
        assert_eq!(Direction::from_delta(0, 0), None);
        for d in Direction::ALL {
            assert_eq!(Direction::from_delta(d.delta().0, d.delta().1), Some(d));
            let (c, s) = (d.angle().cos(), d.angle().sin());
            assert_eq!((c.round() as i32, s.round() as i32), d.delta(), "{d:?}");
        }
    }

    #[test]
    fn distances() {
        let (a, b) = (Point::new(0, 0), Point::new(3, 4));
        assert_eq!(DistanceAlg::Pythagoras.distance2d(a, b), 5.0);
        assert_eq!(DistanceAlg::PythagorasSquared.distance2d(a, b), 25.0);
        assert_eq!(DistanceAlg::Manhattan.distance2d(a, b), 7.0);
        assert_eq!(DistanceAlg::Chebyshev.distance2d(a, b), 4.0);
    }
}
