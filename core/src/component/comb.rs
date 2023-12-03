use crate::{
    axis::*,
    component::{
        attrs,
        struc::*,
        view::{Edge, Element, StrucView},
    },
    construct::{space::*, Error, Type},
};
use serde::Serialize;

#[derive(Clone, Default, Serialize)]
pub struct TransformValue {
    pub allocs: Vec<usize>,
    pub bases: Vec<f32>,
    pub allowances: Vec<f32>,
    pub offset: [f32; 2],
}

impl TransformValue {
    pub fn length(&self) -> f32 {
        self.bases
            .iter()
            .chain(self.allowances.iter())
            .chain(self.offset.iter())
            .sum()
    }

    pub fn allowance_length(&self) -> f32 {
        self.allowances.iter().sum()
    }

    pub fn assigns(&self) -> Vec<f32> {
        self.bases
            .iter()
            .zip(self.allowances.iter())
            .map(|(&b, &a)| a + b)
            .collect()
    }
}

#[derive(Clone)]
pub enum StrucComb {
    Single {
        name: String,
        proto: StrucProto,
        view: StrucView,
        trans: Option<DataHV<TransformValue>>,
    },
    Complex {
        name: String,
        tp: Type,
        combs: Vec<StrucComb>,

        intervals: DataHV<Vec<i32>>,
        i_bases: DataHV<Vec<f32>>,
        i_allowances: DataHV<Vec<f32>>,
        offset: DataHV<[f32; 2]>,
    },
}

impl StrucComb {
    pub fn new_single(name: String, proto: StrucProto) -> Self {
        Self::Single {
            name,
            view: StrucView::new(&proto),
            proto,
            trans: None,
        }
    }

    pub fn new_complex(name: String, tp: Type, combs: Vec<StrucComb>) -> Self {
        Self::Complex {
            name,
            tp,
            combs,
            intervals: Default::default(),
            i_bases: Default::default(),
            i_allowances: Default::default(),
            offset: Default::default(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Single { name, .. } => name,
            Self::Complex { name, .. } => name,
        }
    }

    pub fn white_area(&self) -> DataHV<[f32; 2]> {
        // fn process(comb: &StrucComb, axis: Axis, place: Place) -> f32 {
        //     match comb {
        //         StrucComb::Single { trans, .. } => {
        //             trans.as_ref().unwrap().hv_get(axis).offset[match place {
        //                 Place::Start => 0,
        //                 Place::End => 1,
        //                 Place::Mind => unreachable!(),
        //             }]
        //         }
        //         StrucComb::Complex { tp, combs, .. } => match tp {
        //             Type::Scale(c_axis) => {
        //                 if *c_axis == axis {
        //                     match place {
        //                         Place::Start => process(&combs[0], axis, place),
        //                         Place::End => process(combs.last().unwrap(), axis, place),
        //                         Place::Mind => unreachable!(),
        //                     }
        //                 } else {
        //                     combs
        //                         .iter()
        //                         .map(|c| process(c, axis, place))
        //                         .reduce(|a, b| a.max(b))
        //                         .unwrap()
        //                 }
        //             }
        //             Type::Surround(_) => todo!(),
        //             Type::Single => unreachable!(),
        //         },
        //     }
        // }

        match self {
            Self::Single { trans, .. } => trans.as_ref().unwrap().map(|t| t.offset),
            Self::Complex { offset, .. } => offset.clone(),
        }
    }

    // pub fn read_edge_element(&self, axis: Axis, place: Place) -> Vec<Element> {
    //     self.get_edge(axis, place).unwrap().to_elements(axis, place)
    // }

    pub fn get_char_box(&self) -> WorkBox {
        let mut char_box = WorkBox::new(WorkPoint::zero(), WorkPoint::splat(1.0));
        match self {
            Self::Single { proto, .. } => {
                if let Some([minx, miny, maxx, maxy]) = proto.get_attr::<attrs::CharBox>() {
                    char_box.min = char_box.min.max(WorkPoint::new(minx, miny));
                    char_box.max = char_box.max.min(WorkPoint::new(maxx, maxy));
                }
            }
            _ => {}
        }
        char_box
    }

    // pub fn get_edge(&self, axis: Axis, place: Place) -> Result<Edge, Error> {
    //     // pub fn get_edge_in_surround(
    //     //     &self,
    //     //     surround: DataHV<Place>,
    //     //     secondary: &StrucComb,
    //     //     axis: Axis,
    //     //     place: Place,
    //     // ) -> Result<Edge, Error> {
    //     //     match self {
    //     //         StrucComb::Single { view, name, .. } => {
    //     //             let area = *view
    //     //                 .surround_area(surround)
    //     //                 .ok_or(Error::Surround {
    //     //                     place: surround,
    //     //                     comp: name.clone(),
    //     //                 })?
    //     //                 .hv_get(axis.inverse());
    //     //             let surround = *surround.hv_get(axis.inverse());
    //     //             let max_index = view
    //     //                 .real_size()
    //     //                 .map(|i| i.checked_sub(1).unwrap_or_default());
    //     //             let segment = match place {
    //     //                 Place::Start => 0,
    //     //                 Place::End => *max_index.hv_get(axis),
    //     //                 _ => unreachable!(),
    //     //             };

    //     //             let edge1 = if surround != Place::End {
    //     //                 view.read_edge_in(axis, 0, area[0], segment, place)
    //     //             } else {
    //     //                 Default::default()
    //     //             };
    //     //             let edge2 = if surround != Place::Start {
    //     //                 view.read_edge_in(
    //     //                     axis,
    //     //                     area[1],
    //     //                     *max_index.hv_get(axis.inverse()),
    //     //                     segment,
    //     //                     place,
    //     //                 )
    //     //             } else {
    //     //                 Default::default()
    //     //             };

    //     //             Ok(edge1
    //     //                 .connect(secondary.get_edge(axis, place)?)
    //     //                 .connect(edge2))
    //     //         }
    //     //         Self::Complex { tp, combs, .. } => match tp {
    //     //             Type::Scale(c_axis) => {
    //     //                 if *c_axis == axis {
    //     //                     if *surround.hv_get(axis.inverse()) == Place::End {
    //     //                         combs[0].get_edge_in_surround(surround, secondary, axis, place)
    //     //                     } else {
    //     //                         combs
    //     //                             .last()
    //     //                             .unwrap()
    //     //                             .get_edge_in_surround(surround, secondary, axis, place)
    //     //                     }
    //     //                 } else {
    //     //                     if *surround.hv_get(axis.inverse()) == Place::End {
    //     //                         Ok(combs[0]
    //     //                             .get_edge_in_surround(surround, secondary, axis, place)?
    //     //                             .connect(
    //     //                                 combs[1..]
    //     //                                     .iter()
    //     //                                     .map(|c| c.get_edge(axis, place))
    //     //                                     .reduce(|e1, e2| Edge::connect_result(e1, e2))
    //     //                                     .unwrap()?,
    //     //                             ))
    //     //                     } else {
    //     //                         Ok(combs[..combs.len() - 1]
    //     //                             .iter()
    //     //                             .map(|c| c.get_edge(axis, place))
    //     //                             .reduce(|e1, e2| Edge::connect_result(e1, e2))
    //     //                             .unwrap()?
    //     //                             .connect(
    //     //                                 combs
    //     //                                     .last()
    //     //                                     .unwrap()
    //     //                                     .get_edge_in_surround(surround, secondary, axis, place)?,
    //     //                             ))
    //     //                     }
    //     //                 }
    //     //             }
    //     //             Type::Surround(c_surround) => {
    //     //                 if c_surround.hv_get(axis).inverse() == place
    //     //                     && *c_surround.hv_get(axis.inverse()) != Place::Mind
    //     //                     && axis == Axis::Horizontal
    //     //                 {
    //     //                     //  ↙X
    //     //                     // 十   Bug in c_surround != surround
    //     //                     //  ↖X
    //     //                     assert_eq!(surround, *c_surround);

    //     //                     let new_combs = if *c_surround.hv_get(axis.inverse()) == Place::Start {
    //     //                         vec![combs[1].clone(), secondary.clone()]
    //     //                     } else {
    //     //                         vec![secondary.clone(), combs[1].clone()]
    //     //                     };
    //     //                     let secondary = StrucComb::new_complex(
    //     //                         "read_edge".to_string(),
    //     //                         Type::Scale(Axis::Vertical),
    //     //                         new_combs,
    //     //                     );

    //     //                     combs[0].get_edge_in_surround(surround, &secondary, axis, place)
    //     //                 } else {
    //     //                     combs[0].get_edge_in_surround(surround, secondary, axis, place)
    //     //                 }
    //     //             }
    //     //             Type::Single => unreachable!(),
    //     //         },
    //     //     }
    //     // }

    //     match self {
    //         StrucComb::Single { view, .. } => Ok(view.read_edge(axis, place)),
    //         StrucComb::Complex { tp, combs, .. } => match tp {
    //             Type::Scale(c_axis) => {
    //                 if *c_axis == axis {
    //                     let c = match place {
    //                         Place::Start => &combs[0],
    //                         Place::End => combs.last().unwrap(),
    //                         Place::Mind => unreachable!(),
    //                     };
    //                     c.get_edge(axis, place)
    //                 } else {
    //                     combs
    //                         .iter()
    //                         .map(|c| c.get_edge(axis, place))
    //                         .reduce(|e1, e2| Edge::connect_result(e1, e2))
    //                         .unwrap()
    //                 }
    //             }
    //             Type::Surround(surround_place) => {
    //                 if surround_place.hv_get(axis).inverse() == place {
    //                     // combs[0].get_edge_in_surround(*surround_place, &combs[1], axis, place)
    //                     todo!()
    //                 } else {
    //                     combs[0].get_edge(axis, place)
    //                 }
    //             }
    //             Type::Single => unreachable!(),
    //         },
    //     }
    // }

    pub fn to_struc(&self, start: WorkPoint) -> StrucWork {
        let mut struc = Default::default();
        self.merge_to(&mut struc, start);
        struc
    }

    pub fn merge_to(&self, struc: &mut StrucWork, start: WorkPoint) -> WorkSize {
        match self {
            Self::Single { proto, trans, .. } => {
                let trans = trans.as_ref().unwrap();
                let offset: WorkVec = trans.map(|t| t.offset[0]).to_array().into();
                struc.merge(proto.to_work_in_assign(
                    DataHV::new(&trans.h.assigns(), &trans.v.assigns()),
                    DataHV::splat(0.06),
                    start + offset,
                ));
                WorkSize::new(trans.h.length(), trans.v.length())
            }
            Self::Complex {
                tp,
                combs,
                i_allowances,
                i_bases,
                offset,
                ..
            } => match tp {
                Type::Scale(axis) => {
                    let mut start_pos = start
                        .to_hv_data()
                        .zip(offset.as_ref())
                        .into_map(|(p, o)| p + o[0])
                        .to_array()
                        .into();

                    let mut interval = i_allowances
                        .hv_get(*axis)
                        .iter()
                        .zip(i_bases.hv_get(*axis).iter())
                        .map(|(a, i)| a + i);

                    combs.iter().fold(WorkSize::zero(), |mut size, c| {
                        let mut advance = c.merge_to(struc, start_pos);
                        *advance.hv_get_mut(*axis) += interval.next().unwrap_or_default();

                        *size.hv_get_mut(*axis) += advance.hv_get(*axis);
                        *size.hv_get_mut(axis.inverse()) = size
                            .hv_get(axis.inverse())
                            .max(*advance.hv_get(axis.inverse()));
                        *start_pos.hv_get_mut(*axis) += *advance.hv_get(*axis);
                        size
                    })
                }
                Type::Surround(_) => todo!(),
                Type::Single => unreachable!(),
            },
        }
    }

    pub fn visual_center(&self, min_len: f32, white_area: bool) -> WorkPoint {
        let struc = self.to_struc(WorkPoint::zero());
        let (center, size) = struc.visual_center(min_len);

        if white_area {
            let white_area = self.white_area();
            center
                .to_hv_data()
                .zip(white_area)
                .zip(size.to_hv_data())
                .into_map(|((p, w), s)| (p * s + w[0]) / (w[0] + s + w[1]))
                .to_array()
                .into()
        } else {
            center
        }
    }

    pub fn name_list(&self) -> Vec<String> {
        let mut list = vec![];
        self.for_each_single(|name, _, _, _| list.push(name.to_string()));
        list
    }

    pub fn for_each_single<F>(&self, f: F)
    where
        F: FnMut(&str, &StrucProto, &StrucView, &Option<DataHV<TransformValue>>),
    {
        fn for_each<F>(comb: &StrucComb, mut f: F) -> F
        where
            F: FnMut(&str, &StrucProto, &StrucView, &Option<DataHV<TransformValue>>),
        {
            match comb {
                StrucComb::Single {
                    name,
                    proto,
                    view,
                    trans,
                } => {
                    f(name, proto, view, trans);
                    f
                }
                StrucComb::Complex { combs, .. } => combs.iter().fold(f, |f, c| for_each(c, f)),
            }
        }

        for_each(&self, f);
    }
}
