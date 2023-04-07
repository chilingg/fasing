use super::space::*;
use crate::{fas_file::AllocateTable, DataHV};

use euclid::{Angle, Point2D};
use num_traits::cast::NumCast;

#[derive(Debug)]
pub enum PaddingPointAttr {
    Line(PointAttribute),
    Box([PointAttribute; 4]),
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct PointAttribute {
    symbols: [char; 3],
}

impl PointAttribute {
    pub const SEPARATOR_SYMBOL: &'static str = ";";

    pub fn new(symbols: [char; 3]) -> Self {
        Self { symbols }
    }

    pub fn negative(connect: char) -> char {
        match connect {
            '1' => '9',
            '2' => '8',
            '3' => '7',
            '4' => '6',
            other => other,
        }
    }

    pub fn padding_next(&self) -> PaddingPointAttr {
        match self.next_connect() {
            '1' | '3' | '9' | '7' => PaddingPointAttr::Box([
                PointAttribute::new(['t', self.this_point(), 't']),
                PointAttribute::new(['b', self.this_point(), 'b']),
                PointAttribute::new(['l', self.this_point(), 'l']),
                PointAttribute::new(['r', self.this_point(), 'r']),
            ]),
            '2' | '8' => PaddingPointAttr::Line(PointAttribute::new(['v', self.this_point(), 'v'])),
            '6' | '4' => PaddingPointAttr::Line(PointAttribute::new(['h', self.this_point(), 'h'])),
            '0' => PaddingPointAttr::Line(PointAttribute::new(['0', self.this_point(), '0'])),
            n => panic!("not next symbol `{}`!", n),
        }
    }

    pub fn symbol_of_type(p_type: Option<KeyPointType>) -> char {
        match p_type {
            Some(p_type) => match p_type {
                KeyPointType::Hide => 'N',
                KeyPointType::Line => 'L',
                KeyPointType::Horizontal => 'H',
                KeyPointType::Vertical => 'V',
                KeyPointType::Mark => 'M',
            },
            None => 'X',
        }
    }

    pub fn symbol_of_connect<T, U>(p1: Option<KeyPoint<T, U>>, p2: Option<KeyPoint<T, U>>) -> char
    where
        T: Clone + Copy + NumCast,
    {
        match p1 {
            Some(p1) => match p2 {
                Some(p2) => {
                    let p1: Point2D<f32, _> = p1.point.cast();
                    let p2: Point2D<f32, _> = p2.point.cast();

                    if p1 == p2 {
                        '0'
                    } else {
                        let angle = (p2 - p1).angle_from_x_axis();

                        if angle == Angle::zero() {
                            '6'
                        } else if angle > Angle::zero() && angle < Angle::frac_pi_2() {
                            '3'
                        } else if angle == Angle::frac_pi_2() {
                            '2'
                        } else if angle > Angle::frac_pi_2() && angle < Angle::pi() {
                            '1'
                        } else if angle == Angle::pi() || angle == -Angle::pi() {
                            '4'
                        } else if angle > -Angle::pi() && angle < -Angle::frac_pi_2() {
                            '7'
                        } else if angle == -Angle::frac_pi_2() {
                            '8'
                        } else {
                            '9'
                        }
                    }
                }
                None => '0',
            },
            None => '0',
        }
    }

    pub fn from_key_point<T, U>(
        previous: Option<KeyPoint<T, U>>,
        current: KeyPoint<T, U>,
        next: Option<KeyPoint<T, U>>,
    ) -> Self
    where
        T: Clone + Copy + NumCast,
        U: Copy,
    {
        let mut symbols = ['\0'; 3];

        symbols[0] = Self::symbol_of_connect(Some(current), previous);
        symbols[1] = Self::symbol_of_type(Some(current.p_type));
        symbols[2] = Self::symbol_of_connect(Some(current), next);

        Self { symbols }
    }

    pub fn front_connect(&self) -> char {
        self.symbols[0]
    }

    pub fn this_point(&self) -> char {
        self.symbols[1]
    }

    pub fn next_connect(&self) -> char {
        self.symbols[2]
    }
}

impl ToString for PointAttribute {
    fn to_string(&self) -> String {
        self.symbols.iter().collect()
    }
}

pub type StrucAttributes = DataHV<Vec<String>>;
pub type StrucAllocates = DataHV<Vec<usize>>;

impl StrucAttributes {
    pub fn all_match_indexes(&self, regex: &regex::Regex) -> DataHV<Vec<usize>> {
        DataHV {
            h: self
                .h
                .iter()
                .enumerate()
                .filter_map(|(i, attr)| match regex.is_match(&attr) {
                    true => Some(i),
                    false => None,
                })
                .collect(),
            v: self
                .v
                .iter()
                .enumerate()
                .filter_map(|(i, attr)| match regex.is_match(&attr) {
                    true => Some(i),
                    false => None,
                })
                .collect(),
        }
    }

    pub fn get_space_allocates(&self, alloc_tab: &AllocateTable) -> StrucAllocates {
        fn generate_allocate(attrs: &Vec<String>, alloc_tab: &AllocateTable) -> Vec<usize> {
            StrucAttributes::compact(
                attrs
                    .iter()
                    .map(|attr| alloc_tab.get_weight(attr))
                    .collect(),
            )
        }

        StrucAllocates {
            h: generate_allocate(&self.h, alloc_tab),
            v: generate_allocate(&self.v, alloc_tab),
        }
    }

    pub fn compact(mut allocs: Vec<usize>) -> Vec<usize> {
        // 分配空间等差化
        let mut temp_sort: Vec<_> = allocs.iter_mut().map(|n| n).collect();
        temp_sort.sort();
        temp_sort.into_iter().fold(None, |pre, n| {
            let result;
            if let Some((map_v, pre_v)) = pre {
                if *n != pre_v {
                    result = Some((map_v + 1, *n));
                    *n = map_v + 1;
                } else {
                    *n = map_v;
                    result = pre;
                }
            } else if *n > 2 {
                result = Some((2usize, *n));
                *n = 2;
            } else {
                result = Some((*n, *n));
            }
            result
        });

        allocs
    }
}
