import { Flex, Dropdown, Tooltip } from 'antd';
import { theme } from 'antd';
const { useToken } = theme;
import { useState, useEffect, useRef } from "react";
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import { Context, STORAGE_ID } from "../../lib/storageld";
import ItemsScrollArea from './Scroll';

function get_char_tree_node(tree, set) {
    if (tree.tp === 'Single') {
        set.add(tree.name);
    } else {
        tree.children.forEach(node => {
            get_char_tree_node(node, set)
        });
    }
}

function CharItem({ char, charDisplay, strokeWidth, selectedChar, setSelectedChar }) {
    const { token } = useToken();
    const [hovered, setHovered] = useState(false);
    const [charNode, setcharNode] = useState(new Set());
    const [charPaths, setCharPaths] = useState();
    const [message, setMessage] = useState("");

    const charNodeRef = useRef();

    function processCharTree(tree) {
        let set = new Set();
        get_char_tree_node(tree, set)

        charNodeRef.current = set;
        setcharNode(set);
        return tree;
    }

    function updatePaths() {
        invoke("get_char_tree", { name: char })
            .then(processCharTree)
            .then(tree => {
                invoke("gen_comp_path", { target: tree })
                    .then(([paths, tree]) => {
                        processCharTree(tree);
                        setCharPaths(paths);
                        setMessage("");
                    })
                    .catch(e => {
                        setCharPaths();
                        if ("Empty" in e) {
                            let set = new Set([e.Empty]);
                            charNodeRef.current = set;
                            setcharNode(set);

                            setMessage(`缺少部件\`${e.Empty}\``)
                        } else if ("AxisTransform" in e) {
                            let { axis, length, base_len } = e.AxisTransform;
                            setMessage(`${axis === "Horizontal" ? "横轴" : "竖轴"}中长度${length.toFixed(3)}无法分配到基础值${base_len}`)
                        } else if ("Surround" in e) {
                            let { tp, comp } = e.Surround;
                            setMessage(`组件\`${comp}\`无法应用于包围格式${tp}中`)
                        }
                    })

            });
    }

    useEffect(() => {
        updatePaths();
        let unlistenSrouceChange = listen("source", (e) => {
            updatePaths()
        });
        let unlistenStrucChange = listen("changed", (e) => {
            if (e.payload == "config" || (e.payload.target === "struc" && charNodeRef.current.has(e.payload.value))) {
                updatePaths();
            }
        });

        return () => {
            unlistenSrouceChange.then(f => f());
            unlistenStrucChange.then(f => f());
        };
    }, [char]);

    function handleClickMenu({ key }) {
        invoke("open_struc_editor", { name: key })
    }

    const menuItems = [...charNode].map(comp => {
        return { label: `编辑 \`${comp}\``, key: comp }
    });

    let [color, background] = hovered | char == selectedChar ? [charDisplay.background, charDisplay.color] : [charDisplay.color, charDisplay.background];

    function toSvg(paths) {
        return paths.map((path, i) => {
            return <polyline
                key={i}
                points={path.flat().map(v => Math.round(v * charDisplay.size))} fill="none" stroke={color}
                // points={path.flat().map(v => (v * charDisplay.size).toFixed(1))} fill="none" stroke={color}
                strokeWidth={Math.round(charDisplay.size * strokeWidth)}
                strokeLinecap="square" />
        })
    }

    return <div>
        <Dropdown menu={{ items: menuItems, onClick: handleClickMenu }} trigger={['contextMenu']} >
            <Tooltip title={message} color={token.colorError}>
                <div style={{ lineHeight: 0 }}>
                    <svg style={{ width: charDisplay.size, height: charDisplay.size, backgroundColor: background }}
                        onMouseEnter={() => setHovered(true)}
                        onMouseLeave={() => setHovered(false)}
                        onClick={() => {
                            if (char !== selectedChar) {
                                setSelectedChar(char);
                            } else {
                                setSelectedChar(undefined);
                            }
                        }}
                    >
                        {charPaths && toSvg(charPaths)}
                    </svg>
                </div>
            </Tooltip>
        </Dropdown >
        {charDisplay.charName && <p style={{ textAlign: 'center', padding: '.2em 0 .6em' }}>{char}</p>}
    </div >
}

function Middle({ charList, charDisplay, strokeWidth, selectedChar, setSelectedChar, isFilter }) {
    const { token } = useToken();
    const offsetRef = useRef(Context.getItem(STORAGE_ID.middle.offset));

    function setOffset(e) {
        if (!isFilter) {
            offsetRef.current = e.target.scrollTop;
            Context.setItem(STORAGE_ID.middle.offset, e.target.scrollTop);
        }
    }

    return <Flex vertical style={{ height: '100%' }}>
        <div style={{ flex: "1", padding: 10, overflow: 'hidden' }} >
            <ItemsScrollArea
                updateArea={[charDisplay.charName]}
                initOffset={isFilter ? 0 : offsetRef.current}
                onScroll={setOffset}
                items={
                    charList.map(item => {
                        return {
                            id: item,
                            data: <CharItem
                                char={item}
                                charDisplay={charDisplay}
                                strokeWidth={strokeWidth}
                                selectedChar={selectedChar}
                                setSelectedChar={setSelectedChar}
                            />
                        }
                    })
                } />
        </div>
        <div style={{ padding: '2px 8px', backgroundColor: token.colorBgContainer }}>{charList.length}</div>
    </Flex>
}

export default Middle;