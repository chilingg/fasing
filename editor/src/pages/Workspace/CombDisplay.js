import { Tips } from "@/widgets/Menu";
import Menu from "@/widgets/Menu";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

import { FORMAT_SYMBOL } from "@/lib/construct";
import style from "@/styles/CombDisplay.module.css";
import * as scurve from "@/lib/struc_curve.js"

const CANVAS_PADDING = 12;
const AREA_LENGTH = 48;
const CANVAS_SIZE = AREA_LENGTH + CANVAS_PADDING * 2;

function transform(pos, size, move) {
    return pos.map((v, i) => (v * size[i] + move[i]) * AREA_LENGTH + CANVAS_PADDING);
}

function componentList(name, constAttr, table, config, list) {
    if (config || constAttr?.format !== "Single") {
        let attrs = ["Single", 0, name];
        let mapTo = config?.replace_list;
        if (mapTo) {
            for (let i = 0; i < attrs.length; ++i) {
                mapTo = mapTo[attrs[i]];
                if (!mapTo) {
                    break;
                }
            }
        }
        if (mapTo && mapTo !== name) {
            componentList(mapTo, table.get(mapTo), table, config, list);
            return;
        }
    }

    if (!constAttr || constAttr.format === "Single") {
        let attrs = ["Single", 0, name];
        let mapTo = config?.replace_list;
        if (mapTo) {
            for (let i = 0; i < attrs.length; ++i) {
                mapTo = mapTo[attrs[i]];
                if (!mapTo) {
                    break;
                }
            }
        }
        list.push(mapTo || name);
    } else {
        constAttr.components.forEach(({ Char, Complex }, inFmt) => {
            if (Char) {
                if (config) {
                    let attrs = [constAttr.format, inFmt, Char];
                    let mapTo = config.replace_list;
                    for (let i = 0; i < attrs.length; ++i) {
                        mapTo = mapTo[attrs[i]];
                        if (!mapTo) {
                            break;
                        }
                    }
                    if (mapTo) {
                        Char = mapTo;
                    }
                }
                componentList(Char, table.get(Char), table, config, list);
            } else {
                componentList(null, Complex, table, config, list);
            }
        })
    }
}

function CombSvg({ name, selected, setSelected, constructTab, config, ...props }) {
    const [strucPaths, setStrucPaths] = useState([]);
    const [strucCurve, setStrucCurveProto] = useState([]);

    const [message, setMessage] = useState(null);
    const [menuPos, setMenuPos] = useState();
    const [menuItem, setMenuItem] = useState([]);
    const [subComps, setSubComps] = useState([]);

    useEffect(() => {
        genstrucPaths();
    }, [constructTab, config]);

    useEffect(() => {
        let unlistenStrucChange = listen("struc_change", (e) => {
            if (subComps.includes(e.payload)) {
                genstrucPaths();
            }
        });

        return () => unlistenStrucChange.then(f => f());
    }, [subComps])

    function setStrucCurve(paths) {
        function getCurveD(infos) {
            const RADIUS = 3;
            const CURVE_CONNECT = 2;

            let stroke_sym = ""
            for (let info of infos.slice(0, -1)) {
                stroke_sym += info.next.dir;
            }

            let d = `M ${infos[0].pos[0]} ${infos[0].pos[1]}`;
            switch (stroke_sym) {
                // case "21":
                //     let ctrl = (infos[2].pos[1] - infos[1].pos[1]) * 0.5;
                //     if (ctrl < CURVE_CONNECT) {
                //         ctrl = 0;
                //     }

                //     d += ` L${infos[1].pos[0]} ${infos[1].pos[1]}`;
                //     d += ` C${infos[1].pos[0]} ${infos[1].pos[1] + ctrl} ${infos[2].pos[0]} ${infos[2].pos[1]} ${infos[2].pos[0]} ${infos[2].pos[1]}`;
                //     break;
                default:
                    for (let i = 1; i < infos.length;) {
                        let pinfo = infos[i];

                        // if (pinfo.next?.dir == '2' && pinfo.pre.dir == '6') {
                        //     let move = [pinfo.pos[0] - pinfo.pre.pos[0], pinfo.pos[1] - pinfo.pre.pos[1]];
                        //     d += ` L${(pinfo.pos[0] - RADIUS).toFixed(3)} ${pinfo.pos[1].toFixed(3)}`;
                        //     d += ` A ${RADIUS} ${RADIUS} 0 0 1 ${(pinfo.pos[0]).toFixed(3)} ${(pinfo.pos[1] + RADIUS).toFixed(3)}`;
                        // } else {
                        //     d += ` L${pinfo.pos[0].toFixed(3)} ${pinfo.pos[1].toFixed(3)}`;
                        // }
                        d += ` L${pinfo.pos[0].toFixed(3)} ${pinfo.pos[1].toFixed(3)}`;

                        i += 1;
                    }
            }
            return d
        }

        let curved = paths.map(path => getCurveD(scurve.getPathInfo(path)));
        setStrucCurveProto(curved)
    }

    function genstrucPaths() {
        invoke("get_struc_comb", { name })
            .then(([struc, names]) => {
                setMenuItem(names.map(cName => {
                    return { text: `编辑 ${cName}`, action: () => invoke("open_struc_editor", { name: cName }) };
                }));
                setSubComps(names);

                if (struc.key_paths.length) {
                    let size = [1, 1];
                    let move = [0, 0];
                    if (config.size) {
                        size = [config.size.h, config.size.v];
                        move = size.map(v => (1 - v) / 2);
                    }

                    let paths = [];
                    for (let i = 0; i < struc.key_paths.length; ++i) {
                        let points = struc.key_paths[i].points;
                        if (points[0]?.p_type === "Hide") {
                            continue;
                        }

                        let polylinePos = [];
                        for (let j = 0; j < points.length; ++j) {
                            let pos = points[j].point;
                            polylinePos.push(transform(pos, [1, 1], move));
                        }
                        paths.push(polylinePos);
                    }

                    setStrucPaths(paths);
                    setStrucCurve(paths);
                    setMessage(null);
                }
            })
            .catch(e => {
                invoke("get_comb_name_list", { name })
                    .then(names => {
                        setMenuItem(names.map(cName => {
                            return { text: `编辑 ${cName}`, action: () => invoke("open_struc_editor", { name: cName }) };
                        }));
                        setSubComps(names);
                    })
                    .catch(e => {
                        if (typeof e === "string") {
                            let missingChar = e.match(/"([^"]+)" is empty!/)
                            if (missingChar && missingChar[1]) {
                                setSubComps([missingChar[1]]);
                                setMenuItem([{ text: `编辑 ${missingChar[1]}`, action: () => invoke("open_struc_editor", { name: missingChar[1] }) }]);
                            }
                        }
                        if (typeof e == "string") {
                            setMessage(e);
                        } else {
                            setMessage(e.message);
                        }
                    });
                setStrucPaths([]);
                setStrucCurve([]);
            });
    }

    let svg = (
        <>
            <svg
                className={style.canvas}
                width={CANVAS_SIZE}
                height={CANVAS_SIZE}
                active={selected ? "" : undefined}
                onClick={() => setSelected(!selected)}
                onContextMenu={e => {
                    setMenuPos({ x: e.clientX, y: e.clientY });
                    e.preventDefault();
                }}
            >
                <g>
                    <rect className={style.referenceLine} x={CANVAS_PADDING} y={CANVAS_PADDING} width={AREA_LENGTH} height={AREA_LENGTH} />
                    <line className={style.referenceLine} x1={CANVAS_SIZE / 2} y1={CANVAS_PADDING} x2={CANVAS_SIZE / 2} y2={CANVAS_SIZE - CANVAS_PADDING} />
                    <line className={style.referenceLine} y1={CANVAS_SIZE / 2} x1={CANVAS_PADDING} y2={CANVAS_SIZE / 2} x2={CANVAS_SIZE - CANVAS_PADDING} />
                </g>
                {/* {strucPaths.map((points, i) => <polyline key={i} className={style.strucLine} points={points.join(' ')} strokeLinecap="round" strokeLinejoin="round" />)} */}
                {strucCurve.map((dattr, i) => <path d={dattr} key={i} className={style.strucLine} strokeLinecap="round" strokeLinejoin="round" ></path>)}
            </svg>
            <Menu items={menuItem} pos={menuPos} close={() => setMenuPos(null)} />
        </>
    )
    return message
        ? <Tips tips={message}>{svg}</Tips>
        : svg
}

export default function CombDisplay({ name, ...props }) {
    return (
        <div className={style.area}>
            <CombSvg name={name} {...props} />
            <p style={{ margin: ".4em 0 .8em" }}>{name}</p>
        </div>
    )
}
