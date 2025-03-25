import { useState, useEffect, useRef } from "react";
import { useImmer } from 'use-immer';

import { theme, Flex, Button, Space, Radio } from 'antd';
const { useToken } = theme;

import { invoke } from '@tauri-apps/api/core';

const SELECT_POINTS = "selPos";
const MOUSE_DOWN_POS = "mouseDownPos";
const MOUSE_POS = "mousePos";
const SELECT_MODE = "selMode";

const OLD_POS = "oldPos";

const MODE_SELECT = "sel";
const MODE_MOVE = "move";

const PICK_PATH_POS = "pickPathPos"

function hitPoints(minPos, maxPos, struc, multiple = false) {
    let hitList = [];
    for (let i = 0; i < struc.paths.length; ++i) {
        let points = struc.paths[i].points;
        for (let j = 0; j < points.length; ++j) {
            let pos = points[j];
            if (pos[0] > minPos.x && pos[0] < maxPos.x && pos[1] > minPos.y && pos[1] < maxPos.y) {
                if (multiple) {
                    hitList.push([i, j]);
                } else {
                    return [i, j];
                }
            }
        }
    }
    return hitList;
}

function distanceLessThan(p1, p2, d) {
    return (p1[0] - p2[0]) ** 2 + (p1[1] - p2[1]) ** 2 < d ** 2;
}

function intersect(p1, p2, pos, offset = 0.001) {
    function cmp(a, b) {
        if (a < b)
            return -1;
        if (a > b)
            return 1;
        return 0;
    }

    let a = p2[1] - p1[1];
    let b = p1[0] - p2[0];

    if (a === 0 && b === 0) {
        return distanceLessThan(p1, pos, offset);
    } else {
        let c = -(p1[0] * a + p1[1] * b);
        if (Math.abs(a * pos[0] + b * pos[1] + c) / Math.sqrt(a ** 2 + b ** 2) < offset) {
            let range_x = [p1[0], p2[0]].sort(cmp);
            let range_y = [p1[1], p2[1]].sort(cmp);
            return range_x[0] - offset < pos[0]
                && pos[0] < range_x[1] + offset
                && range_y[0] - offset < pos[1]
                && pos[1] < range_y[1] + offset;
        }

        return false;
    }
}

const GRID_NUMBERS = [5, 10, 20, 40];

const MARK_SIZE = 12;
const MARK_STYLE = { fill: "none", stroke: "red", strokeWidth: "2" };

function EditingArea({ struc, updateStruc, selectTool }) {
    const areaRef = useRef();
    const [gridIndex, setGridIndex] = useState(1);
    const [workData, setWorkData] = useState(new Map());

    const [viewData, setViewData] = useState();

    function screenToStruc([x, y]) {
        return [(x - viewData.offset.x) / viewData.grid, (y - viewData.offset.y) / viewData.grid];
    }

    function strucToScreen([x, y]) {
        return [x * viewData.grid + viewData.offset.x, y * viewData.grid + viewData.offset.y];
    }

    useEffect(() => {
        setWorkData(new Map());
    }, [selectTool]);

    useEffect(() => {
        if (areaRef.current) {
            let rect = areaRef.current.getBoundingClientRect();
            let length = Math.min(rect.width, rect.height);

            let gSize = Math.max(length / (gridNum + 2), (length - 80) / gridNum);
            let offsetX = (rect.width - gSize * gridNum) / 2;
            let offsetY = (rect.height - gSize * gridNum) / 2;

            setViewData({
                grid: gSize,
                offset: {
                    x: offsetX,
                    y: offsetY
                }
            });
        }
    }, [areaRef.current, gridIndex]);

    function handleWheel(e) {
        let next;
        if (e.deltaY > 0) {
            next = Math.min(GRID_NUMBERS.length - 1, gridIndex + 1);
        } else {
            next = Math.max(0, gridIndex - 1)
        }
        if (next !== gridIndex) {
            setGridIndex(next);
        }
    }

    function handleMouseDown(e) {
        if (e.button === 0) {
            let clickPos = screenToStruc([e.clientX, e.clientY]);
            let clickOffset = 5 / viewData.grid;

            switch (selectTool) {
                case "select":
                    let clickTarget = hitPoints(
                        { x: clickPos[0] - clickOffset, y: clickPos[1] - clickOffset },
                        { x: clickPos[0] + clickOffset, y: clickPos[1] + clickOffset },
                        struc
                    );
                    let newData = new Map(workData);

                    if (clickTarget.length) {
                        let selectPoints = newData.get(SELECT_POINTS) || [];

                        let hit = null;
                        for (let i = 0; i < selectPoints.length; ++i) {
                            if (selectPoints[i][0] === clickTarget[0] && selectPoints[i][1] === clickTarget[1]) {
                                hit = i;
                                break;
                            }
                        }

                        if (hit !== null && e.shiftKey) {
                            newData.set(SELECT_POINTS, selectPoints.filter((ele, i) => i !== hit));
                        } else {
                            if (hit === null) {
                                if (e.shiftKey) {
                                    newData.set(SELECT_POINTS, [...selectPoints, clickTarget]);
                                } else {
                                    newData.set(SELECT_POINTS, [clickTarget]);
                                }
                            }

                            let pos = struc.paths[clickTarget[0]].points[clickTarget[1]].point;
                            newData.set(OLD_POS, pos);

                            newData.set(SELECT_MODE, MODE_MOVE);
                            newData.set(MOUSE_DOWN_POS, clickTarget);
                        }
                    } else {
                        newData.set(MOUSE_DOWN_POS, clickPos);
                        newData.set(SELECT_MODE, MODE_SELECT);
                    }

                    setWorkData(newData);
                    break;
                case "add":
                    let pick = workData.get(PICK_PATH_POS);
                    if (pick) {
                        if (pick.tail) {
                            updateStruc(draft => {
                                let points = draft.paths[pick.index].points;
                                let lastPos = points[points.length - 1];
                                points.push([...lastPos]);
                            });
                        } else {
                            updateStruc(draft => {
                                let points = draft.paths[pick.index].points;
                                points.unshift([...points[0]]);
                            });
                        }
                    } else {
                        if (e.shiftKey) {
                            intersectCheck:
                            for (let i = 0; i < struc.paths.length; ++i) {
                                let points = struc.paths[i].points;
                                if (points.length) {
                                    let startPos = points[0];
                                    let endPos = points[points.length - 1];
                                    if (distanceLessThan(endPos, clickPos, clickOffset)) {
                                        updateStruc(draft => {
                                            draft.paths[i].points.push(clickPos);
                                        });

                                        let newData = new Map(workData);
                                        newData.set(PICK_PATH_POS, { index: i, tail: true });
                                        setWorkData(newData);
                                        break intersectCheck;
                                    } else if (distanceLessThan(startPos, clickPos, clickOffset)) {
                                        updateStruc(draft => {
                                            draft.paths[i].points.unshift(clickPos);
                                        });

                                        let newData = new Map(workData);
                                        newData.set(PICK_PATH_POS, { index: i, tail: false });
                                        setWorkData(newData);
                                        break intersectCheck;
                                    } else {
                                        let p1 = points[0];
                                        for (let j = 1; j < points.length; ++j) {
                                            let p2 = points[j];
                                            if (intersect(p1, p2, clickPos, clickOffset)) {
                                                updateStruc(draft => {
                                                    draft.paths[i].points.splice(j, 0, clickPos);
                                                });
                                                break intersectCheck;
                                            }
                                            p1 = p2;
                                        }
                                    }
                                }
                            }
                        } else {
                            let newData = new Map(workData);
                            newData.set(PICK_PATH_POS, { index: struc.paths.length, tail: true });
                            setWorkData(newData);

                            updateStruc(draft => {
                                draft.paths.push({ hide: false, points: [clickPos, clickPos] });
                            });
                        }
                    }
                    break;
                default:
                    console.error(`Unknow select tool: ${selectTool}`);
            }
        }
    }

    function handleMouseMove(e) {
        if (!viewData) return;

        let cursorPos = screenToStruc([e.clientX, e.clientY]);

        switch (selectTool) {
            case "select":
                switch (workData.get(SELECT_MODE)) {
                    case MODE_SELECT:
                        let newData = new Map(workData);
                        newData.set(MOUSE_POS, cursorPos);
                        setWorkData(newData);
                        break;
                    case MODE_MOVE:
                        if (e.ctrlKey) {
                            let oldPos = workData.get(OLD_POS);
                            if (Math.abs(cursorPos[0] - oldPos[0]) > Math.abs(cursorPos[1] - oldPos[1])) {
                                cursorPos[1] = oldPos[1];
                            } else {
                                cursorPos[0] = oldPos[0];
                            }
                        }

                        let targetPosIndex = workData.get(MOUSE_DOWN_POS);
                        let targetPos = struc.paths[targetPosIndex[0]].points[targetPosIndex[1]];
                        let translate = [cursorPos[0] - targetPos[0], cursorPos[1] - targetPos[1]];
                        let selectPoints = workData.get(SELECT_POINTS);

                        updateStruc(draft => {
                            selectPoints.forEach(([i, j]) => {
                                for (let k = 0; k < 2; ++k) {
                                    draft.paths[i].points[j][k] += translate[k];
                                }
                            })
                        });

                        break;
                }
                break;
            case "add":
                let pick = workData.get(PICK_PATH_POS);
                if (pick) {
                    updateStruc(draft => {
                        let points = draft.paths[pick.index].points;
                        points[pick.tail ? points.length - 1 : 0] = [cursorPos[0], cursorPos[1]];
                    })
                }
                break;
            default:
                console.error(`Unknow select tool: ${selectTool}`);
        }
    }

    function handleMouseUp(e) {
        if (e.button === 0) {
            let endPos = screenToStruc([e.clientX, e.clientY]);
            const startPos = workData.get(MOUSE_DOWN_POS);
            const selectPoints = workData.get(SELECT_POINTS) || [];

            switch (selectTool) {
                case "select":
                    let newData = new Map(workData);
                    switch (workData.get(SELECT_MODE)) {
                        case MODE_SELECT:
                            let minPos = { x: Math.min(startPos[0], endPos[0]), y: Math.min(startPos[1], endPos[1]) };
                            let maxPos = { x: Math.max(startPos[0], endPos[0]), y: Math.max(startPos[1], endPos[1]) };
                            let selectTargets = hitPoints(minPos, maxPos, struc, true);

                            if (e.shiftKey) {
                                let addList = [];
                                let removeIndexs = [];
                                selectTargets = selectTargets.forEach(pIndex => {
                                    let ok;
                                    for (let i = 0; i < selectPoints.length; ++i) {
                                        ok = selectPoints[i][0] === pIndex[0] && selectPoints[i][1] === pIndex[1];
                                        if (ok) {
                                            removeIndexs.push(i);
                                            break;
                                        }
                                    }
                                    if (!ok) {
                                        addList.push(pIndex);
                                    }
                                });
                                newData.set(SELECT_POINTS, selectPoints.filter((ele, i) => !removeIndexs.includes(i)).concat(addList));
                            } else {
                                newData.set(SELECT_POINTS, selectTargets);
                            }
                            break;
                        case MODE_MOVE:
                            break;
                    }

                    newData.delete(MOUSE_DOWN_POS);
                    newData.delete(MOUSE_POS);
                    newData.delete(SELECT_MODE);
                    setWorkData(newData);
                    break;
                case "add":
                    break;
                default:
                    console.error(`Unknow select tool: ${selectTool}`);
            }
        }
    }

    function alignStrucValue(axis) {
        let selPoints = workData.get(SELECT_POINTS);
        if (selPoints.length > 1) {
            let alignValue = struc.paths[selPoints[0][0]].points[selPoints[0][1]][axis];
            selPoints.slice(1).forEach(([i, j]) => {
                alignValue = (alignValue + struc.paths[i].points[j][axis]) / 2;
            });
            updateStruc(draft => {
                selPoints.forEach(([i, j]) => draft.paths[i].points[j][axis] = alignValue)
            });
        }
    }

    function moveStrucValue({ x = 0, y = 0 }) {
        let selPoints = workData.get(SELECT_POINTS);
        if (selPoints.length > 0) {
            updateStruc(draft => {
                selPoints.forEach(([i, j]) => {
                    draft.paths[i].points[j][0] += x;
                    draft.paths[i].points[j][1] += y;
                })
            });
        }
    }

    function handleKeyUp(e) {
        if (!e.altKey && !e.ctrlKey && !e.shiftKey) {
            switch (selectTool) {
                case "select":
                    switch (e.key) {
                        case 'c':
                            alignStrucValue(0);
                            break;
                        case 'e':
                            alignStrucValue(1);
                            break;
                        case "Delete":
                            let selPoints = workData.get(SELECT_POINTS);
                            updateStruc(draft => {
                                let changePath = new Set();
                                selPoints.forEach(([i, j]) => {
                                    changePath.add(i);
                                    draft.paths[i].points[j] = null;
                                });
                                changePath.forEach(i => {
                                    draft.paths[i].points = draft.paths[i].points.filter(p => p !== null);
                                })
                                draft.paths = draft.paths.filter(path => path.points.length > 1);
                            });

                            let newData = new Map(workData);
                            newData.delete(SELECT_POINTS);
                            setWorkData(newData);
                            break;
                        case 'a':
                            setWorkData(new Map());
                            break;
                        case 'h':
                            let target = workData.get(SELECT_POINTS);
                            let changePath = new Set();
                            target.forEach(([i, j]) => {
                                changePath.add(i);
                            });
                            updateStruc(draft => changePath.forEach(i => draft.paths[i].hide = !draft.paths[i].hide))
                            break;
                        case "ArrowUp":
                            moveStrucValue({ y: -1 });
                            break;
                        case "ArrowDown":
                            moveStrucValue({ y: 1 });
                            break;
                        case "ArrowLeft":
                            moveStrucValue({ x: -1 });
                            break;
                        case "ArrowRight":
                            moveStrucValue({ x: 1 });
                            break;
                    }
                    break;
                case "add":
                    switch (e.key) {
                        case 'v':
                            setWorkData(new Map());
                            break;
                        case "Escape":
                            let pick = workData.get(PICK_PATH_POS);
                            if (pick) {
                                let newData = new Map(workData);
                                newData.delete(PICK_PATH_POS);
                                setWorkData(newData);

                                updateStruc(draft => {
                                    let points = draft.paths[pick.index].points;
                                    if (points.length === 2) {
                                        draft.paths.splice(pick.index, 1);
                                    } else {
                                        if (pick.tail) {
                                            points.pop()
                                        } else {
                                            points.shift()
                                        }
                                    }
                                })
                            }
                            break;
                    }
                    break;
            }
        }
    }

    const selectPoints = workData.get(SELECT_POINTS);

    let selectBox = undefined;
    if (workData.get(SELECT_MODE) == MODE_SELECT && workData.get(MOUSE_POS)) {
        let startPos = strucToScreen(workData.get(MOUSE_DOWN_POS));
        let endPos = strucToScreen(workData.get(MOUSE_POS));
        if (startPos && endPos) {
            let minPos = { x: Math.min(startPos[0], endPos[0]), y: Math.min(startPos[1], endPos[1]) };
            let maxPos = { x: Math.max(startPos[0], endPos[0]), y: Math.max(startPos[1], endPos[1]) };
            selectBox = (
                <rect
                    x={minPos.x}
                    y={minPos.y}
                    width={maxPos.x - minPos.x}
                    height={maxPos.y - minPos.y}
                    style={{ fill: "transparent", stroke: "cyan", strokeWidth: 1 }}
                />
            )
        }
    }

    let gridNum = GRID_NUMBERS[gridIndex];

    return <svg style={{ flex: 1 }} ref={areaRef} tabIndex={0}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onKeyUp={handleKeyUp}
        onWheel={handleWheel}
    >
        {viewData && <>
            <g>{Array.from({ length: gridNum + 1 }, (_, i) => {
                let pos = strucToScreen([i, i]);
                let origin = strucToScreen([0, 0]);
                let endPos = strucToScreen([gridNum, gridNum]);

                return <g key={`backLine${i}`}>
                    <line x1={origin[0]} y1={pos[1]} x2={endPos[0]} y2={pos[1]} stroke="#cccccc" strokeWidth="1" />
                    <line x1={pos[0]} y1={origin[1]} x2={pos[0]} y2={endPos[1]} stroke="#cccccc" strokeWidth="1" />
                </g>
            })}</g>
            {struc.paths.map((path, i) => {
                let points = path.points.map(p => strucToScreen(p))

                if (path.hide) {
                    return <polyline key={`hidePath${i}`} points={points.flat()} {...MARK_STYLE} />
                } else {
                    return <g key={`path${i}`}>
                        <polyline points={points.flat()} fill="none" stroke="black" strokeWidth="6" />
                        {points.map((p, i) => {
                            let markSize = MARK_SIZE;
                            if (i === 0) {
                                markSize *= 2;
                            }
                            return <rect key={`mark${i}`} x={p[0] - markSize / 2} y={p[1] - markSize / 2} width={markSize} height={markSize} {...MARK_STYLE} />
                        })}
                    </g>
                }
            })}
        </>}
        <g>
            {selectPoints && selectPoints.map(([i, j], index) => {
                const HALF = 4;
                let p = strucToScreen(struc.paths[i].points[j]);
                return <rect key={index} x={p[0] - HALF} y={p[1] - HALF} width={HALF * 2} height={HALF * 2} fill="cyan" />
            })}
        </g>
        {selectBox}
    </svg>
}

export default function Editor() {
    const { token } = useToken();

    const [name, setName] = useState();
    const [struc, updateStruc] = useImmer({ paths: [], attrs: [] });

    const [curTool, setCurTool] = useState("select");

    function alignStruc(struc) {
        let xs = new Set();
        let ys = new Set();
        struc.paths.forEach(path => path.points.forEach(point => {
            point[0] = Math.round(point[0]);
            xs.add(point[0]);
            point[1] = Math.round(point[1]);
            ys.add(point[1]);
        }));

        let minX = Math.min(...xs);
        let minY = Math.min(...ys);
        struc.paths.forEach(path => path.points.forEach(point => {
            point[0] -= minX;
            point[1] -= minY;
        }));
    }

    const TOOLS = [
        {
            label: "选择",
            value: "select",
            shortcut: "v",
            action: () => setCurTool("select")
        }, {
            label: "添加",
            value: "add",
            shortcut: "a",
            action: () => setCurTool("add")

        }, {
            label: "对齐",
            shortcut: "n",
            action: () => updateStruc(alignStruc)

        }, {
            label: "保存",
            shortcut: "s",
            action: () => {
                let newStruc = JSON.parse(JSON.stringify(struc));
                alignStruc(newStruc);

                updateStruc(draft => draft = newStruc);
                invoke("save_struc", { name: name, struc: newStruc }).catch(e => console.error(e));
            }
        }
    ]

    function handleKeyUp(e) {
        if (!e.altKey && !e.ctrlKey && !e.shiftKey) {
            TOOLS.forEach(tool => e.key == tool.shortcut && tool.action())
        }
    }

    useEffect(() => {
        invoke("get_struc_editor_data")
            .then(data => {
                setName(data[0]);
                updateStruc(draft => draft = data[1]);
            });

        window.addEventListener("keyup", handleKeyUp);
        return () => window.removeEventListener("keyup", handleKeyUp);
    }, []);

    return <Flex style={{ height: '100vh' }}>
        <EditingArea struc={struc} updateStruc={updateStruc} selectTool={curTool} />
        <Space size="middle" direction="vertical" style={{ backgroundColor: token.colorBgBase, padding: token.containerPadding }}>
            <Radio.Group
                optionType="button"
                value={curTool}
                onChange={e => e.target.value !== curTool && setCurTool(e.target.value)}
                options={TOOLS.slice(0, 2).map(tool => {
                    return { label: `${tool.label} (${tool.shortcut})`, value: tool.value }
                })}
            />
            {TOOLS.slice(2).map(tool => <Button key={tool.label} size="small" onClick={tool.action}>{`${tool.label} (${tool.shortcut})`}</Button>)}
        </Space>
    </Flex>
}