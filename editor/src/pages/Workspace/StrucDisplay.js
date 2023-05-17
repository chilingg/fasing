import { invoke } from "@tauri-apps/api/tauri";
import { useState, useEffect } from "react";

import style from "@/styles/StrucDisplay.module.css";

const CANVAS_SIZE = 160;
const CANVAS_PADDING = 24;
const AREA_LENGTH = CANVAS_SIZE - CANVAS_PADDING * 2;

const UNREALD_POS_TYPE_H = new Set();
UNREALD_POS_TYPE_H.add("Mark");
UNREALD_POS_TYPE_H.add("Vertical");
const UNREALD_POS_TYPE_V = new Set();
UNREALD_POS_TYPE_V.add("Mark");
UNREALD_POS_TYPE_H.add("Horizontal");
const UNREALD_POS_TYPE = [UNREALD_POS_TYPE_H, UNREALD_POS_TYPE_V];

const MARK_SIZE = 6;

function AllocMarks({ axisValues }) {
    return (
        (axisValues[0].length || axisValues[1].length) && (
            <g>
                {axisValues[0].map(([pos, color], i) => {
                    return <line key={`x${i}`} className={style.referenceLine} x1={pos} y1={0} x2={pos} y2={CANVAS_SIZE} style={{ stroke: color }} />
                })}
                {axisValues[1].map(([pos, color], i) => {
                    return <line key={`x${i}`} className={style.referenceLine} y1={pos} x1={0} y2={pos} x2={CANVAS_SIZE} style={{ stroke: color }} />
                })}
            </g>
        )
    )
}

function Marks({ type, index, options, ...props }) {
    if (type === "Hide") {
        if (options.has("hide")) {
            let points = props.points.map(pos => {
                let x = pos.x.toFixed(3);
                let y = pos.y.toFixed(3);
                return `${x} ${y}`
            }).join(',');
            return (
                <polyline key={index} className={style.mark} points={points} />
            );
        }
    } else {
        let length = props.start ? MARK_SIZE * 2 : MARK_SIZE;
        let transX = props.x.toFixed(3);
        let transY = props.y.toFixed(3);
        let x1 = (transX - length / 2);
        let y1 = (transY - length / 2);

        switch (type) {
            case "Line":
                return options.has("point") && <rect key={index} className={style.mark} x={x1} y={y1} width={length} height={length}></rect>;
            case "Mark":
                return options.has("mark") && (
                    <g key={index}>
                        <line className={style.mark} x1={x1} y1={y1} x2={x1 + length} y2={y1 + length} />
                        <line className={style.mark} x1={x1 + length} y1={y1} x2={x1} y2={y1 + length} />
                    </g>
                );
            case "Horizontal":
                return options.has("mark") && <line key={index} className={style.mark} x1={transX} y1={y1} x2={transX} y2={y1 + length} />
            case "Vertical":
                return options.has("mark") && <line key={index} className={style.mark} x1={x1} y1={transY} x2={x1 + length} y2={transY} />
            default:
                throw new Error(`Undefine mark type ${type}`);
        }
    }
}

export function getRuleLight(weight) {
    const UPPER_LIMIT = 6;
    let level = weight > UPPER_LIMIT ? UPPER_LIMIT : weight;
    return 50 / UPPER_LIMIT * level;
}

function getAllocateColor(table, attr) {
    for (let i = 0; i < table.length; ++i) {
        if (attr.match(table[i].regex)) {
            if (table[i].color === null) {
                break;
            } else {
                return `hsl(${table[i].color} 100% ${getRuleLight(table[i].weight)}%)`;
            }
        }
    }
    return null;
}

function StrucSvg({ name, struc, markingOption, allocateTab }) {
    const [attributes, setAttronites] = useState([[], []]);

    useEffect(() => {
        invoke("get_struc_attribute", { name }).then(attrs => setAttronites([attrs.h, attrs.v]));
    }, [name, struc, allocateTab]);

    // let size = [0, 0];
    let strucPaths = [];
    let marks = [];
    let axisValues = [new Map(), new Map()];

    let scale = AREA_LENGTH;
    let translate = CANVAS_PADDING;

    try {
        if (!struc?.key_paths?.length) {
            let error = new Error("Struc is empty!");
            error.name = "Empty Struc";
            throw error;
        }

        for (let i = 0; i < struc.key_paths.length; ++i) {
            let points = struc.key_paths[i].points;
            let polylinePos = [];

            for (let j = 0; j < points.length; ++j) {
                let pos = [points[j].point[0] * scale + translate, points[j].point[1] * scale + translate,];

                // size[0] = Math.max(pos[0], size[0]);
                // size[1] = Math.max(pos[1], size[1]);

                if (points[0]?.p_type !== "Hide") {
                    marks.push({ type: points[j].p_type, start: j === 0, x: pos[0], y: pos[1] });
                }
                polylinePos.push({ x: pos[0], y: pos[1] });

                for (let axis = 0; axis < 2; ++axis) {
                    if (!UNREALD_POS_TYPE[axis].has(points[j].p_type)) {
                        axisValues[axis].set(pos[axis], 0);
                    }
                }
            }

            if (points[0]?.p_type === "Hide") {
                marks.push({ type: "Hide", points: polylinePos });
            } else {
                strucPaths.push(polylinePos);
            }
        }

        for (let axis = 0; axis < 2; ++axis) {
            axisValues[axis] = [...axisValues[axis]]
                .sort((a, b) => {
                    if (a[0] < b[0]) {
                        return -1;
                    }
                    if (a[0] > b[0]) {
                        return 1;
                    }
                    return 0;
                })
                .slice(1)
                .filter((item, i) => {
                    item[1] = getAllocateColor(allocateTab, attributes[axis][i] || "");
                    return item[1];
                })
        }

        // let realSize = [0, 0];
        // for (let axis = 0; axis < 2; ++axis) {
        //     let attrIndex = 0;
        //     let space = false;
        //     for (let i = 0; i <= size[axis]; ++i) {
        //         if (axisTypes[axis].get(i) === true) {
        //             let match;
        //             if (space) {
        //                 match = getAllocateValue(allocateTab, attributes[axis][attrIndex] || "");
        //             } else {
        //                 match = { alloc: 0, color: null };
        //                 space = true;
        //             }
        //             realSize[axis] += match.alloc;
        //         }
        //     }
        // }

        return (
            <svg className={style.canvas} width={CANVAS_SIZE} height={CANVAS_SIZE}>
                {/* <g>
                    <rect className={style.referenceLine} x={CANVAS_PADDING} y={CANVAS_PADDING} width={AREA_LENGTH} height={AREA_LENGTH} />
                    <line className={style.referenceLine} x1={CANVAS_SIZE / 2} y1={CANVAS_PADDING} x2={CANVAS_SIZE / 2} y2={CANVAS_SIZE - CANVAS_PADDING} />
                    <line className={style.referenceLine} y1={CANVAS_SIZE / 2} x1={CANVAS_PADDING} y2={CANVAS_SIZE / 2} x2={CANVAS_SIZE - CANVAS_PADDING} />
                </g> */}
                {markingOption.has("allocate") && <AllocMarks axisValues={axisValues} />}
                {strucPaths.map((points, i) => (
                    <polyline key={i} className={style.strucLine} points={points.map(pos => {
                        let x = pos.x.toFixed(3);
                        let y = pos.y.toFixed(3);
                        return `${x} ${y}`
                    }).join(',')} />
                ))}
                <g>
                    {
                        marks.map((mark, i) => Marks({ index: i, options: markingOption, ...mark }))
                    }
                </g>
            </svg>
        );
    } catch (error) {
        let msg = `Painting struc error: ${error.name}, ${error.message}`;

        let Error;
        if (error.name !== "Empty Struc") {
            Error = (
                <foreignObject x={CANVAS_PADDING} y={CANVAS_PADDING} width={AREA_LENGTH} height={AREA_LENGTH}>
                    <p className={style.errorText}>{msg}</p>
                </foreignObject>
            )
        }

        // throw error
        return (
            <svg className={style.canvas} width={CANVAS_SIZE} height={CANVAS_SIZE}>
                {Error}
            </svg>
        );
    }
}

export default function StrucDisplay({ name, ...props }) {
    return (
        <div className={style.area}>
            <StrucSvg name={name} {...props} />
            <p>{name}</p>
        </div>
    )
}
