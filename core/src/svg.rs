use crate::{algorithm::NORMAL_OFFSET, construct::space::*};

#[derive(Debug, Clone, Copy)]
// 相对路径
pub enum Draw {
    C {
        p1: WorkVec,
        p2: WorkVec,
        p3: WorkVec,
    },
    L(WorkVec),
    H(f32),
    V(f32),
}

impl Draw {
    pub fn line(vec: WorkVec) -> Self {
        if vec.x.abs() < NORMAL_OFFSET {
            Self::V(vec.y)
        } else if vec.y.abs() < NORMAL_OFFSET {
            Self::H(vec.x)
        } else {
            Self::L(vec)
        }
    }
}

pub struct Path {
    pub start: WorkPoint,
    pub commands: Vec<Draw>,
}

impl Path {
    pub fn new(start: WorkPoint, commands: Vec<Draw>) -> Self {
        Self { start, commands }
    }

    pub fn rect(centre: WorkPoint, length: f32) -> Self {
        Self {
            start: (centre - WorkPoint::splat(length / 2.0)).to_point(),
            commands: vec![
                Draw::H(length),
                Draw::V(length),
                Draw::H(-length),
                Draw::V(-length),
            ],
        }
    }

    pub fn line_of(&mut self, vec: WorkVec) {
        self.commands.push(Draw::line(vec));
    }

    pub fn extend(&mut self, length: f32) {
        match *self.commands.last().unwrap() {
            Draw::C { p3, p2, .. } => self
                .commands
                .push(Draw::line((p3 - p2).normalize() * length)),
            Draw::H(len) => {
                *self.commands.last_mut().unwrap() = Draw::H((len.abs() + length) * len.signum())
            }
            Draw::V(len) => {
                *self.commands.last_mut().unwrap() = Draw::V((len.abs() + length) * len.signum())
            }
            Draw::L(v) => *self.commands.last_mut().unwrap() = Draw::L(v + v.normalize() * length),
        }
    }

    pub fn end_in(&self) -> WorkPoint {
        self.commands
            .iter()
            .fold(self.start, |pos, command| match command {
                Draw::C { p3, .. } => pos + *p3,
                Draw::H(len) => pos + WorkVec::new(*len, 0.0),
                Draw::V(len) => pos + WorkVec::new(0.0, *len),
                Draw::L(v) => pos + *v,
            })
    }

    pub fn reverse(&self) -> Self {
        let start = self.end_in();
        let commands = self
            .commands
            .iter()
            .rev()
            .map(|com| match *com {
                Draw::C { p1, p2, p3 } => Draw::C {
                    p1: p2 - p3,
                    p2: p1 - p3,
                    p3: -p3,
                },
                Draw::H(len) => Draw::H(-len),
                Draw::V(len) => Draw::V(-len),
                Draw::L(v) => Draw::L(-v),
            })
            .collect();
        Self { start, commands }
    }

    pub fn connect(&mut self, other: Self) {
        self.commands.extend(other.commands);
    }

    pub fn to_svg_label(&self, size: WorkSize, class: &str) -> String {
        let mut commands: String = self
            .commands
            .iter()
            .map(|cmd| match cmd {
                Draw::C { p1, p2, p3 } => {
                    format!(
                        "c {:.0} {:.0}, {:.0} {:.0}, {:.0} {:.0} ",
                        p1.x * size.width,
                        p1.y * size.height,
                        p2.x * size.width,
                        p2.y * size.height,
                        p3.x * size.width,
                        p3.y * size.height
                    )
                }
                Draw::H(len) => format!("h {:.0} ", len * size.width),
                Draw::V(len) => format!("v {:.0} ", len * size.height),
                Draw::L(vec) => format!("l {:.0} {:.0} ", vec.x * size.width, vec.y * size.height),
            })
            .collect();

        let end = self.end_in();
        if (self.start - end).length() < NORMAL_OFFSET {
            commands.push('z');
        }

        format!(
            "<path class=\"{}\" d=\"M {:.0} {:.0} {}\"></path>",
            class,
            self.start.x * size.width,
            self.start.y * size.height,
            commands
        )
    }
}

pub fn to_svg_img(paths: &Vec<Path>, size: IndexSize) -> String {
    let head = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" >\n",
        size.width, size.height
    );
    let class = "st0";
    let style = format!(
        "<style type=\"text/css\">.{}{{fill:#000000;stroke-width:0}}</style>\n",
        class
    );
    let paths: String = paths
        .iter()
        .map(|path| path.to_svg_label(WorkSize::new(size.width as f32, size.height as f32), class))
        .collect();

    format!("{head}{style}{paths}</svg>")
}
