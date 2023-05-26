import { invoke } from "@tauri-apps/api/tauri";
import { useState, useEffect, useRef } from "react";

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

function SizeGrid({ width, height }) {
    let size = [width > 2 ? width - 1 : 0, height > 2 ? height - 1 : 0];
    let unit = size.map(n => AREA_LENGTH / (n + 1));
    let axisLines = [[], []];
    for (let axis = 0; axis < 2; ++axis) {
        for (let i = 1; i <= size[axis]; ++i) {
            if (axis === 0) {
                axisLines[axis].push(<line
                    key={`x${i}`}
                    className={style.sizeGrid}
                    x1={i * unit[axis] + CANVAS_PADDING}
                    y1={CANVAS_PADDING}
                    x2={i * unit[axis] + CANVAS_PADDING}
                    y2={AREA_LENGTH + CANVAS_PADDING}
                />);
            } else {
                axisLines[axis].push(<line
                    key={`y${i}`}
                    className={style.sizeGrid}
                    y1={i * unit[axis] + CANVAS_PADDING}
                    x1={CANVAS_PADDING}
                    y2={i * unit[axis] + CANVAS_PADDING}
                    x2={AREA_LENGTH + CANVAS_PADDING}
                />);
            }
        }
    }
    return (<g>
        <rect className={style.sizeGrid} x={CANVAS_PADDING} y={CANVAS_PADDING} width={AREA_LENGTH} height={AREA_LENGTH} />
        {axisLines[0]}
        {axisLines[1]}
    </g>)
}

function AllocMarks({ axisValues }) {
    return (
        (axisValues[0].length || axisValues[1].length) && (
            <g>
                {axisValues[0].map(([pos, color], i) => {
                    return <line key={`x${i}`} className={style.referenceLine} x1={pos} y1={0} x2={pos} y2={CANVAS_SIZE} style={{ stroke: color }} />
                })}
                {axisValues[1].map(([pos, color], i) => {
                    return <line key={`y${i}`} className={style.referenceLine} y1={pos} x1={0} y2={pos} x2={CANVAS_SIZE} style={{ stroke: color }} />
                })}
            </g>
        )
    )
}

function Marks({ type, options, transform, ...props }) {
    if (type === "Hide") {
        if (options.has("hide")) {
            let points = props.points.map(pos => {
                let newPos = transform(pos);
                return `${newPos.x} ${newPos.y}`
            }).join(',');
            return (
                <polyline className={style.mark} points={points} />
            );
        }
    } else {
        let length = props.start ? MARK_SIZE * 2 : MARK_SIZE;
        let transPos = transform({ x: props.x, y: props.y });
        let x1 = (transPos.x - length / 2);
        let y1 = (transPos.y - length / 2);

        switch (type) {
            case "Line":
                return options.has("point") && <rect className={style.mark} x={x1} y={y1} width={length} height={length}></rect>;
            case "Mark":
                return options.has("mark") && (
                    <g>
                        <line className={style.mark} x1={x1} y1={y1} x2={x1 + length} y2={y1 + length} />
                        <line className={style.mark} x1={x1 + length} y1={y1} x2={x1} y2={y1 + length} />
                    </g>
                );
            case "Horizontal":
                return options.has("mark") && <line className={style.mark} x1={transPos.x} y1={y1} x2={transPos.x} y2={y1 + length} />
            case "Vertical":
                return options.has("mark") && <line className={style.mark} x1={x1} y1={transPos.y} x2={x1 + length} y2={transPos.y} />
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

function getAllocateValueAndColor(table, attr, hovered) {
    for (let i = 0; i < table.length; ++i) {
        if (attr.match(table[i].regex)) {
            if (table[i].color === null) {
                return [null, table[i].weight];
            } else {
                let light = getRuleLight(table[i].weight);
                if (hovered) {
                    light = 100 - light;
                }
                return [`hsl(${table[i].color} 100% ${light}%)`, table[i].weight];
            }
        }
    }
    return [null, 1];
}

function StrucSvg({ name, struc, markingOption, allocateTab, hovered }) {
    const [attributes, setAttronites] = useState([[], []]);

    useEffect(() => {
        invoke("get_struc_attribute", { name }).then(attrs => setAttronites([attrs.h, attrs.v]));
    }, [name, struc, allocateTab]);

    let size = [0, 0];
    let strucPaths = [];
    let marks = [];
    let axisValues = [new Map(), new Map()];
    let axisMapTo = [new Map(), new Map()];

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
                let pos = points[j].point;

                size[0] = Math.max(pos[0], size[0]);
                size[1] = Math.max(pos[1], size[1]);

                if (points[0]?.p_type !== "Hide") {
                    marks.push({ type: points[j].p_type, start: j === 0, x: pos[0], y: pos[1] });
                }
                polylinePos.push({ x: pos[0], y: pos[1] });

                for (let axis = 0; axis < 2; ++axis) {
                    if (!axisValues[axis].has(pos[axis]) || !axisValues[axis].get(pos[axis])) {
                        axisValues[axis].set(pos[axis], !UNREALD_POS_TYPE[axis].has(points[j].p_type));
                    }
                }
            }

            if (points[0]?.p_type === "Hide") {
                marks.push({ type: "Hide", points: polylinePos });
            } else {
                strucPaths.push(polylinePos);
            }
        }

        let realSize = [0, 0];
        for (let axis = 0; axis < 2; ++axis) {
            let curAttrIndex = 0;
            for (let i = 0; i <= size[axis]; ++i) {
                if (axisValues[axis].get(i) === true) {
                    if (curAttrIndex === 0) {
                        axisMapTo[axis].set(i, 0);
                    } else {
                        let color, curPos;
                        [color, curPos] = getAllocateValueAndColor(allocateTab, attributes[axis][curAttrIndex - 1] || "", hovered);
                        realSize[axis] += curPos;
                        axisMapTo[axis].set(i, realSize[axis]);
                        axisValues[axis].set(i, color);
                    }
                    ++curAttrIndex;
                } else {
                    axisMapTo[axis].set(i, undefined);
                }
            }
        }

        let proPos = -1;
        for (let axis = 0; axis < 2; ++axis) {
            for (let i = 0; i <= size[axis]; ++i) {
                if (axisMapTo[axis].get(i) === undefined) {
                    let next = i + 1;
                    for (; next <= size[axis]; ++next) {
                        if (axisMapTo[axis].get(next) !== undefined) {
                            break;
                        }
                    }

                    if (next <= size[axis]) {
                        axisMapTo[axis].set(i, (proPos + axisMapTo[axis].get(next)) * 0.5);
                    } else {
                        axisMapTo[axis].set(i, proPos + 0.5);
                    }
                } else {
                    proPos = axisMapTo[axis].get(i);
                }
            }
        }

        let scale = [AREA_LENGTH / (realSize[0] || 1), AREA_LENGTH / (realSize[1] || 1)];
        let translate = [
            (realSize[0] ? 0 : AREA_LENGTH / 2) + CANVAS_PADDING,
            (realSize[1] ? 0 : AREA_LENGTH / 2) + CANVAS_PADDING
        ];

        function transform(pos) {
            return { x: (axisMapTo[0].get(pos.x) * scale[0] + translate[0]).toFixed(3), y: (axisMapTo[1].get(pos.y) * scale[1] + translate[1]).toFixed(3) }
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
                .filter(item => {
                    return typeof item[1] === "string";
                })
                .map(item => [axisMapTo[axis].get(item[0]) * scale[axis] + translate[axis], item[1]])
        }

        return (
            <svg className={style.canvas} width={CANVAS_SIZE} height={CANVAS_SIZE}>
                {/* <g>
                    <rect className={style.referenceLine} x={CANVAS_PADDING} y={CANVAS_PADDING} width={AREA_LENGTH} height={AREA_LENGTH} />
                    <line className={style.referenceLine} x1={CANVAS_SIZE / 2} y1={CANVAS_PADDING} x2={CANVAS_SIZE / 2} y2={CANVAS_SIZE - CANVAS_PADDING} />
                    <line className={style.referenceLine} y1={CANVAS_SIZE / 2} x1={CANVAS_PADDING} y2={CANVAS_SIZE / 2} x2={CANVAS_SIZE - CANVAS_PADDING} />
                </g> */}
                {hovered && <SizeGrid width={realSize[0]} height={realSize[1]} />}
                {markingOption.has("allocate") && <AllocMarks axisValues={axisValues} transform={transform} />}
                {strucPaths.map((points, i) => (
                    <polyline key={i} className={style.strucLine} points={points.map(pos => {
                        let newPos = transform(pos);
                        return `${newPos.x} ${newPos.y}`
                    }).join(',')} />
                ))}
                <g>
                    {
                        marks.map((mark, i) => <Marks key={i} options={markingOption} transform={transform} {...mark} />)
                    }
                </g>
            </svg>
        );
    } catch (error) {
        let msg = `Painting struc error in \`${name}\`: ${error.name}, ${error.message}`;

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
    const [hovered, setHovered] = useState(false);

    return (
        <div className={style.area} onMouseEnter={() => setHovered(true)} onMouseLeave={() => setHovered(false)}>
            <StrucSvg name={name} hovered={hovered} {...props} />
            <p>{name}</p>
        </div>
    )
}
