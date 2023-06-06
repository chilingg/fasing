import Head from 'next/head';
import App from '../_app';

import { getStrucInfo, Marks } from '../Workspace/StrucDisplay';

import { RadioLabel } from '@/widgets/Selection';
import { Vertical } from '@/widgets/Line';
import { Button, ActionBtn } from '@/widgets/Button';

import { useEffect, useRef, useState } from 'react';
import { useImmer } from 'use-immer';
import { invoke } from "@tauri-apps/api/tauri";

import style from "@/styles/StrucEditor.module.css";

const MARKING_LIST = new Set([
    "point",
    "mark",
    "hide"
]);

const VIEW_PADDING = 0.1;
const VIEW_SIZE = 1 + 2 * VIEW_PADDING;

const TOOL_SELECT = {
    label: "选择",
    shortcut: "V",

}
const TOOL_ADD = {
    label: "添加",
    shortcut: "A",

}
const TOOL_SAVE = {
    label: "保存",
    shortcut: "S",

}
const TOOL_NORMALIZATION = {
    label: "标准",
    shortcut: "N",
}

function getToolLabel(tool) {
    return `${tool.label}(${tool.shortcut})`;
}

function strucNormalization(struc) {

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

const MODE_SELECT = "sel";
const MODE_MOVE = "move";

export function SvgEditorArea({ struc, selectTool, updateStruc, setChanged, setCurTool }) {
    const areaRef = useRef();
    const [workData, setWorkData] = useState(new Map());

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

                    if (hit === null) {
                        if (e.shiftKey) {
                            newData.set(SELECT_POINTS, [...selectPoints, clickTarget]);
                        } else {
                            newData.set(SELECT_POINTS, [clickTarget]);
                        }
                        newData.set(SELECT_MODE, MODE_MOVE);
                        newData.set(MOUSE_DOWN_POS, clickTarget);
                    } else {
                        if (e.shiftKey) {
                            let newSelPos = selectPoints.filter((ele, i) => i !== hit);
                            newData.set(SELECT_POINTS, newSelPos);
                        } else {
                            newData.set(SELECT_MODE, MODE_MOVE);
                            newData.set(MOUSE_DOWN_POS, selectPoints[hit]);
                        }
                    }
                } else {
                    newData.set(MOUSE_DOWN_POS, clickPos);
                    newData.set(SELECT_MODE, MODE_SELECT);
                }

                setWorkData(newData);
                break;
            case "add":
                break;
            default:
                console.error(`Unknow select tool: ${selectTool}`);
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
                        setChanged();

                        break;
                }
                break;
            case "add":
                break;
            default:
                console.error(`Unknow select tool: ${selectTool}`);
        }
    }

    function handleMouseUp(e) {
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
                    default:
                        console.error(`Unknow select tool: ${selectTool}`);
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

    function alignStrucValue(axis) {
        let selPoints = workData.get(SELECT_POINTS);
        console.log(selPoints)
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

    function handleKeyUp(e) {
        console.log(e.keyCode)
        if (!e.altKey && !e.ctrlKey && !e.shiftKey) {
            switch (selectTool) {
                case "select":
                    switch (e.keyCode) {
                        case 67:
                            alignStrucValue(0);
                            setChanged();
                            break;
                        case 69:
                            alignStrucValue(1);
                            setChanged();
                            break;
                        case 46:
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
                            setChanged();
                            break;
                        case TOOL_ADD.shortcut.charCodeAt():
                            setCurTool("add");
                            break;
                    }
                    break;
                case "add":
                    if (e.keyCode === TOOL_SELECT.shortcut.charCodeAt()) {
                        setCurTool("select");
                    }
                    break;
            }
        }
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

    return (
        <svg
            ref={areaRef}
            className={style.editorArea}
            viewBox={`-${VIEW_PADDING} -${VIEW_PADDING} ${VIEW_SIZE} ${VIEW_SIZE}`}
            onMouseDown={handleMouseDown}
            onMouseUp={handleMouseUp}
            onMouseMove={handleMouseMove}
            tabIndex={0}
            onKeyUp={handleKeyUp}
        >
            <rect width={1} height={1} x={0} y={0} className={style.pageArea} />
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
    )
}

export function Editor() {
    const [name, setName] = useState();
    const [struc, updateStruc] = useImmer({ key_paths: [], tags: [] });

    const [curTool, setCurTool] = useState("select");
    const [changed, setChanged] = useState(false);

    useEffect(() => {
        invoke("get_struc_editor_data")
            .then(data => {
                setName(data[0]);
                updateStruc(draft => draft = data[1]);
            })
    }, []);

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

    return (
        <div className={style.background}>
            <SvgEditorArea
                struc={struc}
                selectTool={curTool}
                updateStruc={updateStruc}
                setChanged={() => !changed && setChanged(true)}
                setCurTool={setCurTool}
            />
            <div className={style.toolsArea}>
                <Vertical>
                    <RadioLabel items={toolBtns} currents={curTool} vertical={true} onChange={(e, value) => {
                        if (value !== curTool) {
                            setCurTool(value);
                        }
                    }} />
                    <hr />
                    <ActionBtn active={changed ? "active" : undefined}>{getToolLabel(TOOL_SAVE)}</ActionBtn>
                    <Button>{getToolLabel(TOOL_NORMALIZATION)}</Button>
                    <Button>退出</Button>
                </Vertical>
            </div>
        </div>
    )
}

export default function Index() {
    return (
        <>
            <Head>
                <title>Struc Editor</title>
                <meta name="description" content="Generated by create next app" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="icon" href="/favicon.ico" />
            </Head>
            <App Component={Editor}></App>
        </>
    )
}