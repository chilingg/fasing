import { invoke } from "@tauri-apps/api/tauri";
import { useState } from "react";

import style from "@/styles/StrucDisplay.module.css";

const CANVAS_SIZE = 160;
const CANVAS_PADDING = 24;
const AREA_LENGTH = CANVAS_SIZE - CANVAS_PADDING * 2;

const UNREALD_POS_TYPE_H = new Set();
UNREALD_POS_TYPE_H.add("Mark");
const UNREALD_POS_TYPE_V = new Set();
UNREALD_POS_TYPE_V.add("Mark");
const UNREALD_POS_TYPE = [UNREALD_POS_TYPE_H, UNREALD_POS_TYPE_V];

const MARK_SIZE = 6;

function Marks({ type, index, translate, scale, axisMapTo, ...props }) {
    if (type === "Hide") {
        let points = props.points.map(pos => {
            let x = (axisMapTo[0].get(pos.x) * scale[0] + translate[0]).toFixed(3);
            let y = (axisMapTo[1].get(pos.y) * scale[1] + translate[1]).toFixed(3);
            return `${x} ${y}`
        }).join(',');
        return (
            <polyline key={index} className={style.mark} points={points} />
        );
    } else {
        let length = props.start ? MARK_SIZE * 2 : MARK_SIZE;
        let transX = (axisMapTo[0].get(props.x) * scale[0] + translate[0]).toFixed(3);
        let transY = (axisMapTo[1].get(props.y) * scale[1] + translate[1]).toFixed(3);
        let x1 = (transX - length / 2);
        let y1 = (transY - length / 2);

        switch (type) {
            case "Line":
                return <rect key={index} className={style.mark} x={x1} y={y1} width={length} height={length}></rect>;
            case "Mark":
                return (
                    <g key={index}>
                        <line className={style.mark} x1={x1} y1={y1} x2={x1 + length} y2={y1 + length} />
                        <line className={style.mark} x1={x1 + length} y1={y1} x2={x1} y2={y1 + length} />
                    </g>
                );
            case "Horizontal":
                return <line key={index} className={style.mark} x1={transX} y1={y1} x2={transX} y2={y1 + length} />
            case "Vertical":
                return <line key={index} className={style.mark} x1={x1} y1={transY} x2={x1 + length} y2={transY} />
            default:
                throw new Error(`Undefine mark type ${type}`);
        }
    }
}

function StrucSvg({ struc }) {
    let size = [0, 0];
    let strucPaths = [];
    let marks = [];
    let axisTypes = [new Map(), new Map()];
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

                for (let k = 0; k < 2; ++k) {
                    if (!axisTypes[k].has(pos[k]) || !axisTypes[k].get(pos[k])) {
                        axisTypes[k].set(pos[k], !UNREALD_POS_TYPE[k].has(points[j].p_type));
                    }
                }
            }

            if (points[0]?.p_type === "Hide") {
                marks.push({ type: "Hide", points: polylinePos });
            } else {
                strucPaths.push(polylinePos);
            }
        }

        let realSize = [...size];
        for (let axis = 0; axis < 2; ++axis) {
            let offset = 0;
            for (let i = 0; i <= size[axis]; ++i) {
                if (axisTypes[axis].get(i) === true) {
                    axisMapTo[axis].set(i, i - offset);
                } else {
                    axisMapTo[axis].set(i, i - offset - 0.5);
                    offset += 1;
                    realSize[axis] -= 1;
                }
            }
        }

        let scale = [AREA_LENGTH / (realSize[0] || 1), AREA_LENGTH / (realSize[1] || 1)];
        let translate = [
            (realSize[0] ? 0 : AREA_LENGTH / 2) + CANVAS_PADDING,
            (realSize[1] ? 0 : AREA_LENGTH / 2) + CANVAS_PADDING
        ];

        return (
            <svg className={style.canvas} width={CANVAS_SIZE} height={CANVAS_SIZE}>
                <g>
                    <rect className={style.referenceLine} x={CANVAS_PADDING} y={CANVAS_PADDING} width={AREA_LENGTH} height={AREA_LENGTH} />
                    <line className={style.referenceLine} x1={CANVAS_SIZE / 2} y1={0} x2={CANVAS_SIZE / 2} y2={CANVAS_SIZE} />
                    <line className={style.referenceLine} y1={CANVAS_SIZE / 2} x1={0} y2={CANVAS_SIZE / 2} x2={CANVAS_SIZE} />
                </g>
                {strucPaths.map((points, i) => (
                    <polyline key={i} className={style.strucLine} points={points.map(pos => {
                        let x = (axisMapTo[0].get(pos.x) * scale[0] + translate[0]).toFixed(3);
                        let y = (axisMapTo[1].get(pos.y) * scale[1] + translate[1]).toFixed(3);
                        return `${x} ${y}`
                    }).join(',')} />
                ))}
                <g>
                    {
                        marks.map((mark, i) => Marks({ index: i, scale: scale, translate: translate, axisMapTo: axisMapTo, ...mark }))
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

        return (
            <svg className={style.canvas} width={CANVAS_SIZE} height={CANVAS_SIZE}>
                {Error}
            </svg>
        );
    }
}

export default function StrucDisplay({ name }) {
    const [struc, setStruc] = useState();

    if (!struc) {
        invoke("get_struc_proto", { name: name })
            .then(s => setStruc(s));
    }

    return (
        <div className={style.area}>
            <StrucSvg struc={struc} />
            <p>{name}</p>
        </div>
    )
}
