import Footer from "./Footer";
import Settings from "./Settings";
import StrucDisplay from "./StrucDisplay";
import SettingPanel from "./SettingPanel";

import { ItemsScrollArea } from "@/widgets/Scroll";
import { Spacer } from "@/widgets/Space";
import { Button, SelectBtn } from "@/widgets/Button";
import { SelectionLabel } from "@/widgets/Selection";
import { Horizontal, Vertical } from "@/widgets/Line";
import Input from "@/widgets/Input";
import { ContentPanel, Tips } from "@/widgets/Menu";
import { HuePicker } from "@/widgets/ColorPicker";

import { useEffect, useRef, useState } from "react";

import { STORAGE_ID, Context } from "@/lib/storageId";
import style from "@/styles/ComponentWorkspace.module.css"

const MARK_OPTIONS = [
    "point",
    "mark",
    "hide",
    "allocate"
];

const FILTER_TYPE_OPTIONS = [
    "empty",
    "valid",
    "single",
    "complex",
]

function ColorBtn({ color, setColor, switchColor }) {
    const [pos, setPos] = useState(null);

    return (
        <div
            className={style.colorBtn}
            style={{ backgroundColor: color ? `hsl(${color.h}, ${color.s}%, ${color.l}%)` : "transparent" }}
            onClick={(e) => {
                setPos({ left: e.clientX - 160, top: e.clientY })
            }}
        >
            <ContentPanel
                pos={pos}
                setClose={() => setPos(null)}
            >
                <Horizontal style={{ overflow: "visible" }}>
                    <HuePicker hue={color === null ? 0 : color.h} setHue={setColor} disabled={color === null ? "disabled" : null} />
                    <Button onClick={() => switchColor(color === null ? true : false)}>{color === null ? "显示" : "隐藏"}</Button>
                </Horizontal>
            </ContentPanel>

        </div>
    );
}

function AllocateSettings({ allocateTab, setAllocateTab }) {
    return (
        <div>
            <table className={style.allocateTab}>
                <thead>
                    <tr>
                        <th>序号</th>
                        <th><SelectBtn
                            checked={allocateTab.find(rule => rule.disabled) ? false : true}
                            onClick={(e, checked) => {
                                setAllocateTab(allocateTab.map(rule => { return { ...rule, disabled: !checked } }))
                            }}
                        /></th>
                        <th>权重</th>
                        <th>过滤</th>
                        <th>规则</th>
                    </tr>
                </thead>
                <tfoot >
                    <tr>
                        <td colSpan="5">总计：{allocateTab.length}</td>
                    </tr>
                </tfoot>
                <tbody>
                    {allocateTab.map((rule, i) => {
                        return (
                            <tr key={i}>
                                <td>{i}</td>
                                <td>
                                    <SelectBtn
                                        checked={rule.disabled ? false : true}
                                        onClick={(e, checked) => {
                                            let newTab = [...allocateTab];
                                            newTab[i].disabled = !checked;
                                            setAllocateTab(newTab);
                                        }}
                                    />
                                </td>
                                <td>{rule.weight}</td>
                                <td>{rule.filter.length === 0 ? "无" : <Tips tips={rule.filter.join('; ')}>有</Tips>}</td>
                                <td>{rule.regex.source}</td>
                            </tr>
                        )
                    })}
                </tbody>
            </table>
        </div>
    )
}

function WorkspaceSettingPanel({ allocateTab, setAllocateTab }) {
    const [openAllocate, setOpenAllocate] = useState(true);
    const [openRegexTest, setOpenRegexTest] = useState(true);
    const [width, setWidth] = useState(360);

    useEffect(() => {
        let w = Context.getItem(STORAGE_ID.compWorkspace.settingPanel.width);
        if (w && w > 100) {
            setWidth(w);
        }
    }, []);

    function handleResize(rect) {
        setWidth(rect.width);
        Context.setItem(STORAGE_ID.compWorkspace.settingPanel.width, rect.width);
    }

    let items = [
        {
            id: "allocate",
            title: "空间分配",
            open: openAllocate,
            setOpen: setOpenAllocate,
            component: (
                <AllocateSettings allocateTab={allocateTab} setAllocateTab={setAllocateTab} />
            )
        },
        {
            id: "tests",
            title: "规则测试",
            open: openRegexTest,
            setOpen: setOpenRegexTest,
            component: (
                <p>规则测试</p>
            )
        },
    ];

    return (
        <SettingPanel
            items={items}
            width={width}
            onResize={handleResize}
        />
    )
}

function WorkspaceSettings({
    filter,
    setFilter,
    markingOption,
    setMarkingOption,
}) {
    let filterOptionList = [
        {
            value: "empty",
            label: "空结构"
        },
        {
            value: "valid",
            label: "非空结构"
        },
        {
            value: "single",
            label: "单字码"
        },
        {
            value: "complex",
            label: "复合码"
        },
    ]

    let markingOptionList = [
        {
            value: "point",
            label: "点注"
        },
        {
            value: "mark",
            label: "标记"
        },
        {
            value: "hide",
            label: "隐线"
        },
        {
            value: "allocate",
            label: "分配"
        }
    ];

    function handleMarkingOptionSet(e, active, value) {
        let newMarkingOp = new Set(markingOption);
        active ? newMarkingOp.add(value) : newMarkingOp.delete(value);
        setMarkingOption(newMarkingOp);
    }

    function handleFilterOptionSet(e, active, value) {
        let newOpiton = new Set(filter.options);
        active ? newOpiton.add(value) : newOpiton.delete(value);
        setFilter({ options: newOpiton, text: filter.text });
    }

    return (
        <Settings>
            <Vertical>
                <Horizontal>
                    <Input label="过滤" value={filter.text} setValue={val => setFilter({ text: val, options: filter.options })} />
                    <SelectionLabel items={filterOptionList} currents={filter.options} onChange={handleFilterOptionSet} />
                    <hr vertical="" />
                    <label>显示</label>
                    <SelectionLabel items={markingOptionList} currents={markingOption} onChange={handleMarkingOptionSet} />
                </Horizontal>
            </Vertical>
        </Settings>
    )
}

export default function ComponentsWorkspace({ compList, allocateTab, setAllocateTab }) {
    const [filter, setFilterProto] = useState({
        text: "",
        options: new Set(FILTER_TYPE_OPTIONS),
    });
    const [markingOption, setMarkingOptionProto] = useState(new Set(MARK_OPTIONS));

    const normalOffsetRef = useRef(Context.getItem(STORAGE_ID.compWorkspace.scrollOffset));

    useEffect(() => {
        let filter = Context.getItem(STORAGE_ID.compWorkspace.setting.filter);
        filter && setFilterProto(filter);
        let markings = Context.getItem(STORAGE_ID.compWorkspace.setting.markings);
        markings && setMarkingOptionProto(markings);
    }, []);

    function setFilter(filter) {
        setFilterProto(filter);
        Context.setItem(STORAGE_ID.compWorkspace.setting.filter, filter);
    }

    function setMarkingOption(options) {
        setMarkingOptionProto(options);
        Context.setItem(STORAGE_ID.compWorkspace.setting.markings, options);
    }

    function isFiltering() {
        return filter.text || filter.options.size != FILTER_TYPE_OPTIONS.length
    }

    function handleScroll(e) {
        if (!isFiltering()) {
            normalOffsetRef.current = e.target.scrollTop;
            Context.setItem(STORAGE_ID.compWorkspace.scrollOffset, e.target.scrollTop);
        }
    }

    let strucItems = [];
    compList.forEach((struc, name) => {
        if (filter.text.length === 0 || filter.text.indexOf(name) !== -1) {
            if (!filter.options.has("empty") && struc.key_paths.length === 0) {
                return
            }
            if (!filter.options.has("valid") && struc.key_paths.length) {
                return
            }
            if (!filter.options.has("single") && name.length === 1) {
                return
            }
            if (!filter.options.has("complex") && name.length !== 1) {
                return
            }
            strucItems.push({
                id: name,
                data: {
                    name,
                    struc,
                    markingOption,
                    allocateTab,
                }
            })
        }
    })

    // Test
    // let name = "⺗";
    // strucItems = [{
    //     id: name,
    //     data: {
    //         name,
    //         struc: compList.get(name),
    //         markingOption,
    //         allocateTab,
    //     }
    // }];

    return (
        <div style={{ display: "flex", flexDirection: "row", height: "100%" }}>
            <div style={{ display: "flex", flex: "1", flexDirection: "column" }}>
                <WorkspaceSettings
                    filter={filter}
                    setFilter={setFilter}
                    markingOption={markingOption}
                    setMarkingOption={setMarkingOption}
                />
                <div style={{ flex: "1" }}>
                    <ItemsScrollArea
                        ItemType={StrucDisplay}
                        items={strucItems}
                        onScroll={handleScroll}
                        initOffset={isFiltering() ? 0 : normalOffsetRef.current}
                    />
                </div>
                <Footer>
                    <Spacer />
                    <p>部件 {isFiltering() ? `${strucItems.length} / ${compList.size}` : compList.size}</p>
                </Footer>
            </div>
            <WorkspaceSettingPanel allocateTab={allocateTab} setAllocateTab={setAllocateTab} />
        </div>
    );
}