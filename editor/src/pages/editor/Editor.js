
import { getStrucInfo, Marks } from '../Workspace/StrucDisplay';

import { RadioLabel } from '@/widgets/Selection';
import { Horizontal, Vertical } from '@/widgets/Line';
import { Button, ActionBtn } from '@/widgets/Button';
import Menu from '@/widgets/Menu';

import { useEffect, useRef, useState } from 'react';
import { useImmer } from 'use-immer';
import { invoke } from "@tauri-apps/api/tauri";

import style from "@/styles/StrucEditor.module.css";
import { Item, List } from '@/widgets/List';

const MARKING_LIST = new Set([
    "point",
    "mark",
    "hide"
]);

const VIEW_PADDING = 0.1;
const VIEW_SIZE = 1 + 2 * VIEW_PADDING;

const TOOL_SELECT = {
    label: "选择",
    shortcut: "v",

}
const TOOL_ADD = {
    label: "添加",
    shortcut: "a",

}
const TOOL_SAVE = {
    label: "保存",
    shortcut: "s",

}
const TOOL_NORMALIZATION = {
    label: "标准",
    shortcut: "n",
}
const TOOL_ALIGN = {
    label: "对齐",
    shortcut: "l",
}

function getToolLabel(tool) {
    return `${tool.label}(${tool.shortcut})`;
}

function distanceLessThan(p1, p2, disrabce) {
    return (p1.x - p2.x) ** 2 + (p1.y - p2.y) ** 2 < disrabce ** 2;
}

function intersect(p1, p2, pos, offset = 0.001) {
    function cmp(a, b) {
        if (a < b)
            return -1;
        if (a > b)
            return 1;
        return 0;
    }

    let a = p2.y - p1.y;
    let b = p1.x - p2.x;

    if (a === 0 && b === 0) {
        return distanceLessThan(p1, pos, offset);
    } else {
        let c = -(p1.x * a + p1.y * b);
        if (Math.abs(a * pos.x + b * pos.y + c) / Math.sqrt(a ** 2 + b ** 2) < offset) {
            let range_x = [p1.x, p2.x].sort(cmp);
            let range_y = [p1.y, p2.y].sort(cmp);
            return range_x[0] - offset < pos.x
                && pos.x < range_x[1] + offset
                && range_y[0] - offset < pos.y
                && pos.y < range_y[1] + offset;
        }

        return false;
    }
}

function hitPoints(minPos, maxPos, struc, multiple = false) {
    let hitList = [];
    for (let i = 0; i < struc.key_paths.length; ++i) {
        let points = struc.key_paths[i].points;
        for (let j = 0; j < points.length; ++j) {
            let pos = points[j].point;
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

const SELECT_POINTS = "selPos";
const MOUSE_DOWN_POS = "mouseDownPos";
const MOUSE_POS = "mousePos";
const SELECT_MODE = "selMode";

const OLD_POS = "oldPos";

const MODE_SELECT = "sel";
const MODE_MOVE = "move";

const PICK_PATH_POS = "pickPathPos"

export function SvgEditorArea({ struc, selectTool, updateStruc, setCurTool, gridNum, setGridIndex }) {
    const areaRef = useRef();
    const [workData, setWorkData] = useState(new Map());
    const [menuPos, setMenuPos] = useState();

    useEffect(() => {
        setWorkData(new Map());
    }, [selectTool]);

    function toWorkCoordinates(pos) {
        let rect = areaRef.current.getBoundingClientRect();
        let ratio;
        let offset;
        if (rect.width > rect.height) {
            ratio = VIEW_SIZE / rect.height;
            offset = [(rect.width * ratio - 1) / 2, VIEW_PADDING];
        } else {
            ratio = VIEW_SIZE / rect.width;
            offset = [VIEW_PADDING, (rect.height * ratio - 1) / 2];
        }
        return { x: (pos.x - rect.x) * ratio - offset[0], y: (pos.y - rect.y) * ratio - offset[1] }
    }

    function ratio() {
        let rect = areaRef.current.getBoundingClientRect();
        if (rect.width > rect.height) {
            return VIEW_SIZE / rect.height;
        } else {
            return VIEW_SIZE / rect.width;
        }
    }

    function handleMouseDown(e) {
        if (e.button === 0) {
            let clickPos = toWorkCoordinates({ x: e.clientX, y: e.clientY });
            let clickOffset = 5 * ratio();

            switch (selectTool) {
                case "select":
                    let clickTarget = hitPoints(
                        { x: clickPos.x - clickOffset, y: clickPos.y - clickOffset },
                        { x: clickPos.x + clickOffset, y: clickPos.y + clickOffset },
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

                            console.log([...newData.get(SELECT_POINTS)])
                            let pos = struc.key_paths[clickTarget[0]].points[clickTarget[1]].point;
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
                                let points = draft.key_paths[pick.index].points;
                                let lastPos = points[points.length - 1];
                                points.push({
                                    p_type: lastPos.p_type,
                                    point: [...lastPos.point]
                                });
                            });
                        } else {
                            updateStruc(draft => {
                                let points = draft.key_paths[pick.index].points;
                                points.unshift({
                                    p_type: points[0].p_type,
                                    point: [...points[0].point]
                                });
                            });
                        }
                    } else {
                        if (e.shiftKey) {
                            intersectCheck:
                            for (let i = 0; i < struc.key_paths.length; ++i) {
                                let points = struc.key_paths[i].points;
                                if (points.length) {
                                    let startPos = points[0].point;
                                    let endPos = points[points.length - 1].point;
                                    if (distanceLessThan({ x: endPos[0], y: endPos[1] }, clickPos, clickOffset)) {
                                        updateStruc(draft => {
                                            draft.key_paths[i].points.push({ p_type: points[0].p_type, point: [clickPos.x, clickPos.y] });
                                        });

                                        let newData = new Map(workData);
                                        newData.set(PICK_PATH_POS, { index: i, tail: true });
                                        setWorkData(newData);
                                        break intersectCheck;
                                    } else if (distanceLessThan({ x: startPos[0], y: startPos[1] }, clickPos, clickOffset)) {
                                        updateStruc(draft => {
                                            draft.key_paths[i].points.unshift({ p_type: points[0].p_type, point: [clickPos.x, clickPos.y] });
                                        });

                                        let newData = new Map(workData);
                                        newData.set(PICK_PATH_POS, { index: i, tail: false });
                                        setWorkData(newData);
                                        break intersectCheck;
                                    } else {
                                        let p1 = points[0].point;
                                        for (let j = 1; j < points.length; ++j) {
                                            let p2 = points[j].point;
                                            if (intersect({ x: p1[0], y: p1[1] }, { x: p2[0], y: p2[1] }, clickPos, clickOffset)) {
                                                updateStruc(draft => {
                                                    draft.key_paths[i].points.splice(j, 0, { p_type: points[0].p_type, point: [clickPos.x, clickPos.y] });
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
                            newData.set(PICK_PATH_POS, { index: struc.key_paths.length, tail: true });
                            setWorkData(newData);

                            updateStruc(draft => {
                                draft.key_paths.push({ closed: false, points: [{ p_type: "Line", point: [clickPos.x, clickPos.y] }, { p_type: "Line", point: [clickPos.x, clickPos.y] }] });
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
        let cursorPos = toWorkCoordinates({ x: e.clientX, y: e.clientY });

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
                            if (Math.abs(cursorPos.x - oldPos[0]) > Math.abs(cursorPos.y - oldPos[1])) {
                                cursorPos.y = oldPos[1];
                            } else {
                                cursorPos.x = oldPos[0];
                            }
                        }

                        let targetPosIndex = workData.get(MOUSE_DOWN_POS);
                        let targetPos = struc.key_paths[targetPosIndex[0]].points[targetPosIndex[1]].point;
                        let translate = [cursorPos.x - targetPos[0], cursorPos.y - targetPos[1]];
                        let selectPoints = workData.get(SELECT_POINTS);

                        updateStruc(draft => {
                            selectPoints.forEach(([i, j]) => {
                                for (let k = 0; k < 2; ++k) {
                                    draft.key_paths[i].points[j].point[k] += translate[k];
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
                        let points = draft.key_paths[pick.index].points;
                        points[pick.tail ? points.length - 1 : 0].point = [cursorPos.x, cursorPos.y];
                    })
                }
                break;
            default:
                console.error(`Unknow select tool: ${selectTool}`);
        }
    }

    function handleMouseUp(e) {
        if (e.button === 0) {
            let endPos = toWorkCoordinates({ x: e.clientX, y: e.clientY });
            const startPos = workData.get(MOUSE_DOWN_POS);
            const selectPoints = workData.get(SELECT_POINTS) || [];

            switch (selectTool) {
                case "select":
                    let newData = new Map(workData);
                    switch (workData.get(SELECT_MODE)) {
                        case MODE_SELECT:
                            let minPos = { x: Math.min(startPos.x, endPos.x), y: Math.min(startPos.y, endPos.y) };
                            let maxPos = { x: Math.max(startPos.x, endPos.x), y: Math.max(startPos.y, endPos.y) };
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
            let alignValue = struc.key_paths[selPoints[0][0]].points[selPoints[0][1]].point[axis];
            selPoints.slice(1).forEach(([i, j]) => {
                alignValue = (alignValue + struc.key_paths[i].points[j].point[axis]) / 2;
            });
            updateStruc(draft => {
                selPoints.forEach(([i, j]) => draft.key_paths[i].points[j].point[axis] = alignValue)
            });
        }
    }

    function moveStrucValue({ x = 0, y = 0 }) {
        let selPoints = workData.get(SELECT_POINTS);
        if (selPoints.length > 1) {
            let moveX = x / gridNum;
            let moveY = y / gridNum;
            selPoints.forEach(([i, j]) => {
                console.log(struc.key_paths[i].points[j].point[0], "+=", moveX);
            })
            updateStruc(draft => {
                selPoints.forEach(([i, j]) => {
                    draft.key_paths[i].points[j].point[0] += moveX;
                    draft.key_paths[i].points[j].point[1] += moveY;
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
                                    draft.key_paths[i].points[j] = null;
                                });
                                changePath.forEach(i => {
                                    draft.key_paths[i].points = draft.key_paths[i].points.filter(p => p !== null);
                                })
                                draft.key_paths = draft.key_paths.filter(path => path.points.length > 1);
                            });

                            let newData = new Map(workData);
                            newData.delete(SELECT_POINTS);
                            setWorkData(newData);
                            break;
                        case TOOL_ADD.shortcut:
                            setCurTool("add");
                            setWorkData(new Map())
                            break;
                        case "ArrowUp":
                            moveStrucValue({ y: -1 });
                    }
                    break;
                case "add":
                    switch (e.key) {
                        case TOOL_SELECT.shortcut:
                            setCurTool("select");
                            setWorkData(new Map());
                            break;
                        case "Escape":
                            let pick = workData.get(PICK_PATH_POS);
                            if (pick) {
                                let newData = new Map(workData);
                                newData.delete(PICK_PATH_POS);
                                setWorkData(newData);

                                updateStruc(draft => {
                                    let points = draft.key_paths[pick.index].points;
                                    if (points.length === 2) {
                                        draft.key_paths.splice(pick.index, 1);
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

    function handleContextMenu(e) {
        setMenuPos({ x: e.clientX, y: e.clientY });
        e.preventDefault();
    }

    function handleWheel(e) {
        let ratio = gridNum / setGridIndex(e.deltaY);
        if (ratio !== 1 && !e.ctrlKey) {
            updateStruc(draft => draft.key_paths.forEach(path => path.points.forEach(kp => {
                kp.point[0] *= ratio;
                kp.point[1] *= ratio;
            })));
        }
    }

    function setKeyPointType(type) {
        let selsPos = workData.get(SELECT_POINTS) || [];
        selsPos.forEach(([i, j]) => updateStruc(draft => {
            draft.key_paths[i].points[j].p_type = type;
        }));
    }

    function isSelecting() {
        return workData.get(SELECT_POINTS)?.length;
    }

    let strucInfo = getStrucInfo(struc);
    const selectPoints = workData.get(SELECT_POINTS);

    let selectBox = undefined;
    if (workData.get(SELECT_MODE) == MODE_SELECT) {
        let startPos = workData.get(MOUSE_DOWN_POS);
        let endPos = workData.get(MOUSE_POS);
        if (startPos && endPos) {
            let minPos = { x: Math.min(startPos.x, endPos.x), y: Math.min(startPos.y, endPos.y) };
            let maxPos = { x: Math.max(startPos.x, endPos.x), y: Math.max(startPos.y, endPos.y) };
            selectBox = (
                <rect
                    x={minPos.x}
                    y={minPos.y}
                    width={maxPos.x - minPos.x}
                    height={maxPos.y - minPos.y}
                    style={{ fill: "transparent", stroke: "cyan", strokeWidth: 1.5 * ratio() }}
                />
            )
        }
    }

    let pointMenuItems = [
        {
            text: "Line",
            action: () => setKeyPointType("Line")
        },
        {
            text: "Horizontal",
            action: () => setKeyPointType("Horizontal")
        },
        {
            text: "Vertical",
            action: () => setKeyPointType("Vertical")
        },
        {
            text: "Mark",
            action: () => setKeyPointType("Mark")
        },
        {
            text: "Hide",
            action: () => setKeyPointType("Hide")
        }
    ];

    let grid = undefined;
    if (gridNum > 1) {
        let unit = 1 / gridNum;
        grid = [];
        for (let i = 1; i < gridNum; ++i) {
            grid.push(i * unit);
        }
    }

    return (
        <>
            <svg
                ref={areaRef}
                className={style.editorArea}
                viewBox={`-${VIEW_PADDING} -${VIEW_PADDING} ${VIEW_SIZE} ${VIEW_SIZE}`}
                tabIndex={0}
                onMouseMove={handleMouseMove}
                onKeyUp={handleKeyUp}
                onMouseDown={handleMouseDown}
                onMouseUp={handleMouseUp}
                onContextMenu={handleContextMenu}
                onWheel={handleWheel}
            >
                <g>
                    <rect width={1} height={1} x={0} y={0} className={style.pageArea} />
                    {grid && grid.map((n, i) => <line key={`h${i}`} className={style.pageArea} x1={n} x2={n} y1={0} y2={1} />)}
                    {grid && grid.map((n, i) => <line key={`v${i}`} className={style.pageArea} y1={n} y2={n} x1={0} x2={1} />)}
                </g>
                {strucInfo.paths.map((points, i) => (
                    <polyline key={i} className={style.strucLine} points={points.map(pos => `${pos.x} ${pos.y}`).join(',')} />
                ))}
                <g>{
                    strucInfo.marks.map((mark, i) => {
                        return <Marks key={i} options={MARKING_LIST} markSize={0.03} className={style.mark} {...mark} />
                    })
                }</g>
                <g>
                    {selectPoints && selectPoints.map(([i, j], index) => {
                        const HALF = 0.006;
                        let p = struc.key_paths[i].points[j].point;
                        return <rect key={index} x={p[0] - HALF} y={p[1] - HALF} width={HALF * 2} height={HALF * 2} fill="cyan" />
                    })}
                </g>
                {selectBox}
            </svg>
            <Menu items={isSelecting() ? pointMenuItems : []} pos={menuPos} close={() => setMenuPos(null)} />
        </>
    )
}

function ConditionAllocs({ inplace, subArea }) {
    // const CASES = [
    //     [false, false, false, false],
    //     [true, false, false, false],
    //     [false, true, false, false],
    //     [false, false, true, false],
    //     [false, false, false, true],
    //     [true, true, false, false],
    //     [true, false, true, false],
    //     [true, false, false, true],
    //     [false, true, true, false],
    //     [false, true, false, true],
    //     [false, false, true, true],

    //     [true, true, true, true],
    //     [false, true, true, true],
    //     [true, false, true, true],
    //     [true, true, false, true],
    //     [true, true, true, false],
    //     [false, false, true, true],
    //     [false, true, false, true],
    //     [false, true, true, false],
    //     [true, false, false, true],
    //     [true, false, true, false],
    //     [true, true, false, false],
    // ]

    return (<div>
        <p>条件分配</p>
        <List direction='column'>
            {inplace.map(([condition, allocs], i) => {
                let allocs_text_h = undefined;
                let check_h = {};
                if (allocs.h.length) {
                    allocs_text_h = allocs.h.join(',');
                    if (allocs.h.length != subArea.width) {
                        check_h.textDecoration = "red wavy underline";
                    }
                }
                let allocs_text_v = undefined;
                let check_v = {};
                if (allocs.v.length) {
                    allocs_text_v = allocs.v.join(',');
                    if (allocs.v.length != subArea.height) {
                        check_v.textDecoration = "red wavy underline";
                    }
                }

                return (<Item key={i} style={{ listStyleType: "disc", marginLeft: "1.5em", paddingBottom: "0.5em", overflow: "visible" }}>
                    <p>{condition}</p>
                    {allocs_text_h && <p style={{ overflow: "visible" }}>h: <span style={check_h}>{allocs_text_h}</span></p>}
                    {allocs_text_v && <p style={{ overflow: "visible" }}>v: <span style={check_v}>{allocs_text_v}</span></p>}
                </Item>)
            })}
        </List>
    </div>)
}

function ReduceAllocs({ reduceList, subArea }) {
    return (<div>
        <p>收缩分配</p>
        {reduceList.h.length && <List direction='column'>{
            reduceList.h.map((allocs, i) => {
                let check = {};
                let allocs_text = allocs.join(',');
                if (allocs.length != subArea.width) {
                    check.textDecoration = "red wavy underline";
                }

                return <Item key={i} style={{ overflow: "visible" }}>h: <span style={check}>{allocs_text}</span></Item>
            })
        }</List>}
        {reduceList.v.length && <List direction='column'>{
            reduceList.v.map((allocs, i) => {
                let check = {};
                let allocs_text = allocs.join(',');
                if (allocs.length != subArea.height) {
                    check.textDecoration = "red wavy underline";
                }

                return <Item key={i} style={{ overflow: "visible" }}>v: <span style={check}>{allocs_text}</span></Item>
            })
        }</List>}
    </div>)
}

function SettingPanel({ struc, unit }) {
    function get_allocs(struc) {
        function map_to(datas) {
            let list = [];
            datas.forEach((r, val) => r && list.push(val))
            list.sort();
            if (list.length) {
                let pre = list[0];
                let temp = []
                for (let i = 0; i < list.length; ++i) {
                    temp.push(list[i] - pre);
                    pre = list[i];
                }
                list = temp.slice(1).map(v => Math.round(v / unit));
            }
            return list;
        }

        let values = getStrucInfo(struc).axisValues;
        return [map_to(values[0]), map_to(values[1])]
    }

    let [hlist, vlist] = get_allocs(struc);

    return (<Vertical>
        <p>横轴 {`分区数：${hlist.length} 长度：${hlist.reduce((a, b) => a + b, 0)}`}</p>
        <p>{hlist.join(',')}</p>
        <p>竖轴 {`分区数：${vlist.length} 长度：${vlist.reduce((a, b) => a + b, 0)}`}</p>
        <p>{vlist.join(',')}</p>
        <hr />
        {struc.attrs?.in_place && <ConditionAllocs inplace={JSON.parse(struc.attrs.in_place)} subArea={{ width: hlist.length, height: vlist.length }} />}
        <hr />
        {struc.attrs?.reduce_alloc && <ReduceAllocs reduceList={JSON.parse(struc.attrs.reduce_alloc)} subArea={{ width: hlist.length, height: vlist.length }} />}
    </Vertical>)
}

const GRID_NUMBERS = [5, 10, 20, 40];

export default function Editor() {
    const [name, setName] = useState();
    const [struc, updateStrucProto] = useImmer({ key_paths: [], tags: [] });

    const [curTool, setCurTool] = useState("select");
    const [changed, setChanged] = useState(false);

    const [gridIndex, setGridIndexProto] = useState(1);

    function setGridIndex(n) {
        let next;
        if (n > 0) {
            next = Math.min(GRID_NUMBERS.length - 1, gridIndex + 1);
        } else {
            next = Math.max(0, gridIndex - 1)
        }
        setGridIndexProto(next);
        return GRID_NUMBERS[next];
    }

    useEffect(() => {
        invoke("get_struc_editor_data", { grid: [GRID_NUMBERS[gridIndex], GRID_NUMBERS[gridIndex]] })
            .then(data => {
                setName(data[0]);
                updateStrucProto(draft => draft = data[1]);
            })
    }, []);

    function updateStruc(f) {
        !changed && setChanged(true);
        updateStrucProto(f);
    }

    let toolBtns = [
        {
            label: getToolLabel(TOOL_SELECT),
            value: "select"
        },
        {
            label: getToolLabel(TOOL_ADD),
            value: "add"
        }
    ];

    let gridNum = GRID_NUMBERS[gridIndex];

    return (
        <Horizontal className={style.background}>
            <SvgEditorArea
                struc={struc}
                selectTool={curTool}
                updateStruc={updateStruc}
                setCurTool={setCurTool}
                gridNum={gridNum}
                setGridIndex={setGridIndex}
            />
            <div className={style.toolsArea}>
                <Vertical>
                    <RadioLabel items={toolBtns} currents={curTool} vertical={true} onChange={(e, value) => {
                        if (value !== curTool) {
                            setCurTool(value);
                        }
                    }} />
                    <hr />
                    <ActionBtn
                        active={changed}
                        onAction={(e, notChanged) => { notChanged || invoke("save_struc_in_cells", { name, struc, unit: [1 / gridNum, 1 / gridNum] }).then(() => setChanged(false)) }}
                    >{getToolLabel(TOOL_SAVE)}</ActionBtn>
                    <Button onClick={() => {
                        invoke("align_cells", { struc, unit: [1 / gridNum, 1 / gridNum] })
                            .then(struc => {
                                updateStruc(draft => draft = struc);
                            });
                    }}>{getToolLabel(TOOL_ALIGN)}</Button>
                    <Button onClick={() => {
                        invoke("normalization", { struc, offset: 0.01 })
                            .then(struc => updateStruc(draft => draft = struc));
                    }}>{getToolLabel(TOOL_NORMALIZATION)}</Button>
                    <Button>退出</Button>
                </Vertical>
            </div>
            <div className={style.settingArea}>
                <SettingPanel struc={struc} unit={1 / gridNum} />
            </div>
        </Horizontal>
    )
}
