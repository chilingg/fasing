import { Tips } from "@/widgets/Menu";
import Menu from "@/widgets/Menu";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

import { FORMAT_SYMBOL } from "@/lib/construct";
import style from "@/styles/CombDisplay.module.css";

const CANVAS_SIZE = 48;
const CANVAS_PADDING = 8;
const AREA_LENGTH = CANVAS_SIZE - CANVAS_PADDING * 2;

function transform(pos, size, move) {
    return pos.map((v, i) => (v * size[i] + move[i]) * AREA_LENGTH + CANVAS_PADDING);
}

function componentList(name, constAttr, table, config, list) {
    if (!constAttr || constAttr.format === "Single") {
        list.push(name);
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

    useEffect(() => {
        let map_name = config?.replace_list["Single"] && config.replace_list["Single"]["0"][name] || name;
        let constAttr = constructTab.get(map_name);
        let compList = [];
        componentList(map_name, constAttr, constructTab, config, compList);

        setMenuItem(compList.map(cName => {
            return { text: `编辑 ${cName}`, action: () => invoke("open_struc_editor", { name: cName }) };
        }));
        genstrucPaths(constAttr);

        let unlistenStrucChange = listen("struc_change", (e) => {
            if (compList.includes(e.payload)) {
                genstrucPaths(constAttr);
            }
        });

        return () => unlistenStrucChange.then(f => f());
    }, [constructTab]);

    function genstrucPaths(cAttr) {
        invoke("get_struc_comb", { name })
            .then(struc => {
                if (struc.key_paths.length) {
                    let size = [1, 1];
                    let move = [0, 0];
                    if (struc.tags.length) {
                        if (cAttr?.format === "Single") {
                            if (struc.tags.includes("top")) {
                                size = [1, 0.5];
                            } else if (struc.tags.includes("bottom")) {
                                size = [1, 0.5];
                                move = [0, 0.5];
                            } else if (struc.tags.includes("left")) {
                                size = [0.5, 1];
                            } else if (struc.tags.includes("right")) {
                                size = [0.5, 1];
                                move = [0.5, 0];
                            }
                        }
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
            .catch(e => setMessage(e));
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
                {strucPaths.map((points, i) => <polyline key={i} className={style.strucLine} points={points.join(' ')} strokeLinecap="square" strokeLinejoin="round" />)}
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
