import { Tips } from "@/widgets/Menu";
import Menu from "@/widgets/Menu";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

import { FORMAT_SYMBOL } from "@/lib/construct";
import style from "@/styles/CombDisplay.module.css";

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
    const [message, setMessage] = useState(null);
    const [menuPos, setMenuPos] = useState();
    // const [constructAttr, setConstructAttr] = useState({ components: [], format: "Single" });
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
                            polylinePos.push(transform(pos, size, move));
                        }
                        paths.push(polylinePos);
                    }

                    setStrucPaths(paths);
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
                {strucPaths.map((points, i) => <polyline key={i} className={style.strucLine} points={points.join(' ')} strokeLinecap="round" strokeLinejoin="round" />)}
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
