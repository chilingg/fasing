import Settings from "./Settings";
import Footer from "./Footer";
import SettingPanel from "./SettingPanel";

import { Horizontal, Vertical } from "@/widgets/Line";
import { ItemsScrollArea } from "@/widgets/Scroll";
import CombDisplay from "./CombDisplay";
import Input from "@/widgets/Input";
import { Selections } from "@/widgets/Selection";
import { Button } from "@/widgets/Button";

import { Context, STORAGE_ID } from "@/lib/storageId";

import { useState, useEffect, useRef } from "react";

const WORK_ID = STORAGE_ID.combWorkspace;

const CHAR_GROUP_LIST = [
    {
        value: "Single",
        label: "单体"
    },
    {
        value: "LeftToRight",
        label: "⿰"
    },
    {
        value: "LeftToMiddleAndRight",
        label: "⿲"
    },
    {
        value: "AboveToBelow",
        label: "⿱"
    },
    {
        value: "AboveToMiddleAndBelow",
        label: "⿳"
    },
    {
        value: "SurroundFromAbove",
        label: "⿵"
    },
    {
        value: "SurroundFromBelow",
        label: "⿶"
    },
    {
        value: "FullSurround",
        label: "⿴"
    },
    {
        value: "SurroundFromUpperRight",
        label: "⿹"
    },
    {
        value: "SurroundFromLeft",
        label: "⿷"
    },
    {
        value: "SurroundFromUpperLeft",
        label: "⿸"
    },
    {
        value: "SurroundFromLowerLeft",
        label: "⿺"
    },
];

function WorkspaceSettings({
    filter,
    setFilter,
    charGroup,
    setCharGroup,
    genCharMembers,
}) {

    function handleCharGroupChange(e, active, value) {
        let list = new Set(charGroup);
        active ? list.add(value) : list.delete(value);
        setCharGroup(list);
    }

    return (
        <Settings>
            <Vertical>
                <Horizontal>
                    <Input label="过滤" value={filter} setValue={setFilter} />
                    <hr vertical="" />
                    <Selections items={CHAR_GROUP_LIST} currents={charGroup} onChange={handleCharGroupChange} />
                    <Button onClick={() => genCharMembers()}>生成</Button>
                </Horizontal>
            </Vertical>
        </Settings>
    )
}

function CharInfo({ char }) {
    return (
        <p>{char}</p>
    )
}

function WorkspaceSettingPanel({ selects }) {
    const [openSelect, setOpenSelect] = useState(true);
    const [openGroup, setOpenGroup] = useState(true);
    const [openConfig, setConfig] = useState(true);
    const [width, setWidth] = useState(360);

    useEffect(() => {
        let w = Context.getItem(WORK_ID.settingPanel.width);
        if (w && w > 100) {
            setWidth(w);
        }
    }, []);

    let items = [
        {
            id: "select",
            title: "选中",
            open: openSelect,
            setOpen: setOpenSelect,
            component: (
                selects.size ? <CharInfo char={selects.values().next().value} /> : <p>未选中</p>
            )
        },
        {
            id: "group",
            title: "字符组",
            open: openGroup,
            setOpen: setOpenGroup,
            component: (
                <p>字符组</p>
            )
        },
        {
            id: "config",
            title: "配置",
            open: openConfig,
            setOpen: setConfig,
            component: (
                <p>配置</p>
            )
        },
    ];

    function handleResize(rect) {
        setWidth(rect.width);
        Context.setItem(WORK_ID.settingPanel.width, rect.width);
    }

    return (
        <SettingPanel
            items={items}
            width={width}
            onResize={handleResize}
        />
    )
}

export default function CombinationWOrkspace({ constructTab }) {
    const [charGroup, setCharGroupProto] = useState(new Set(["Single"]));
    const [filter, setFilter] = useState("");
    const [charMembers, setCharMembers] = useState([]);
    const [selects, setSelectsProto] = useState(new Set());

    const normalOffsetRef = useRef(Context.getItem(WORK_ID.scrollOffset));

    useEffect(() => {
        let group = Context.getItem(WORK_ID.charGroup);

        if (group) {
            setCharGroupProto(group);
            genCharMembersInGroup(group);
        }

        // let sele = Context.getItem(WORK_ID.selects);
        // sele && setSelectsProto(sele);
    }, []);

    function setCharGroup(group) {
        setCharGroupProto(group);
        Context.setItem(WORK_ID.charGroup, group);
    }

    function setSelects(targets) {
        setSelectsProto(targets);
        Context.setItem(WORK_ID.selects, targets);
    }

    function genCharMembersInGroup(group = charGroup) {
        let members = [];
        for (const [name, attrs] of constructTab) {
            if (group.has(attrs.format)) {
                members.push(name)
            }
        }
        setCharMembers(members);
    }

    function handleScroll(e) {
        if (filter.length === 0) {
            normalOffsetRef.current = e.target.scrollTop;
            Context.setItem(WORK_ID.scrollOffset, e.target.scrollTop);
        }
    }

    let charDatas = charMembers.map(char => {
        return {
            id: char,
            data: {
                name: char,
                selected: selects.has(char),
                setSelected: (sele => {
                    // let newSelects = new Set(selects);
                    // sele ? newSelects.add(char) : newSelects.delete(char);
                    // setSelects(newSelects);
                    setSelects(sele ? new Set([char]) : new Set());
                })
            }
        }
    });
    // Test
    // let name = "⺌";
    // charDatas = [
    //     {
    //         id: name,
    //         data: {
    //             name
    //         }
    //     }
    // ];

    return (
        <div style={{ display: "flex", flexDirection: "row", height: "100%" }}>
            <div style={{ display: "flex", flex: "1", flexDirection: "column" }}>
                <WorkspaceSettings
                    filter={filter}
                    setFilter={setFilter}
                    charGroup={charGroup}
                    setCharGroup={setCharGroup}
                    genCharMembers={genCharMembersInGroup}
                />
                <div style={{ flex: "1" }}>
                    <ItemsScrollArea
                        ItemType={CombDisplay}
                        items={charDatas}
                        onScroll={handleScroll}
                        initOffset={filter.length === 0 ? normalOffsetRef.current : 0}
                    />
                </div>
                <Footer>
                    <p>{`${charMembers.length} 字符`}</p>
                </Footer>
            </div>
            <WorkspaceSettingPanel selects={selects} />
        </div>
    )
}