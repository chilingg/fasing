import Footer from "./Footer";
import Settings from "./Settings";
import StrucDisplay from "./StrucDisplay";
import SettingPanel from "./SettingPanel";

import { ItemsScrollArea } from "@/widgets/Scroll";
import { Spacer } from "@/widgets/Space";
import { ActionBtn } from "@/widgets/Button";
import { SelectionLabel } from "@/widgets/Selection";
import { Horizontal, Vertical } from "@/widgets/Line";
import Input from "@/widgets/Input";

import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";

import style from "@/styles/ComponentWorkspace.module.css"

const MARK_OPTIONS = [
    "point",
    "mark",
    "hide",
];

const FILTER_TYPE_OPTIONS = [
    "empty",
    "valid",
    "single",
    "complex",
]

const scrollStrogeId = "ComponentWorkspaceScroll";

function AllocateSettings({ allocateTab, setAllocateTab }) {
    let icon = <div style={{ width: 12, height: 12, backgroundColor: "white" }} />;
    console.log(allocateTab)
    return (
        <table className={style.allocateTab}>
            <thead>
                <tr>
                    <th>{icon}</th>
                    <th>权重</th>
                    <th>规则</th>
                </tr>
            </thead>
            <tfoot >
                <td colspan="4">总计：{allocateTab.length}</td>
            </tfoot>
            <tbody>
                {allocateTab.map(rule => (
                    <tr>
                        <td>{icon}</td>
                        <td>{rule.weight}</td>
                        <td>{rule.regex.source}</td>
                    </tr>
                ))}
            </tbody>
        </table>
    )
}

function WorkspaceSettingPanel({ allocateTab, setAllocateTab }) {
    const [openSelecte, setOpenSelecte] = useState(false)
    const [openAllocate, setOpenAllocate] = useState(true)
    const [openRegexTest, setOpenRegexTest] = useState(true)
    const [width, setWidth] = useState(360);

    function handleResize(rect) {
        invoke("set_context_value", { key: ["settingPanel", "width"], value: rect.width });
    }

    useEffect(() => {
        invoke("get_context_value", { key: ["settingPanel", "width"] })
            .then(width => {
                if (width > 100)
                    setWidth(width);
            })
    }, []);

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
    filterText,
    setFilterText,
    filterOption,
    setFilterOption,
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
        invoke("set_context_value", { key: ["componentWorkspace", "options", "marking"], value: [...newMarkingOp] });
    }

    function handleFilterOptionSet(e, active, value) {
        let newOpiton = new Set(filterOption);
        active ? newOpiton.add(value) : newOpiton.delete(value);
        setFilterOption(newOpiton);
        invoke("set_context_value", { key: ["componentWorkspace", "options", "filter"], value: [...newOpiton] });
    }

    return (
        <Settings>
            <Vertical>
                <Horizontal>
                    <Input label="过滤" value={filterText} setValue={setFilterText} />
                    <SelectionLabel items={filterOptionList} currents={filterOption} onChange={handleFilterOptionSet} />
                    <hr vertical="" />
                    <label>显示</label>
                    <SelectionLabel items={markingOptionList} currents={markingOption} onChange={handleMarkingOptionSet} />
                    <hr vertical="" />
                    <ActionBtn>执行分配</ActionBtn>
                </Horizontal>
            </Vertical>
        </Settings>
    )
}

export default function ComponentsWorkspace() {
    const [compList, setCompList] = useState(new Map());
    const [markingOption, setMarkingOption] = useState(new Set(MARK_OPTIONS));
    const [filterOption, setFilterOption] = useState(new Set(FILTER_TYPE_OPTIONS));
    const [filterText, setFilterText] = useState("");

    const [allocateTab, setAllocateTab] = useState([]);

    const normalOffsetRef = useRef(Number(localStorage.getItem(scrollStrogeId)));

    function loadData() {
        invoke("get_struc_all")
            .then(list => setCompList(new Map(Object.entries(list))));
        invoke("get_allocate_tabel")
            .then(tab => {
                let newTab = tab.map(rule => {
                    rule.regex = new RegExp(rule.regex);
                    return rule;
                });
                setAllocateTab(newTab);
            });
    }

    useEffect(() => {
        let unlisten = listen("source", (e) => {
            switch (e.payload.event) {
                case "load":
                    loadData();
                    break;
            }
        });

        loadData();

        invoke("get_context_value", { key: ["componentWorkspace", "options"] })
            .then(options => {
                if (options) {
                    if (options.marking && options.marking !== markingOption) {
                        setMarkingOption(new Set(options.marking))
                    }
                    if (options.filter && options.filter !== filterOption) {
                        setFilterOption(new Set(options.filter))
                    }
                }
            });

        return () => {
            unlisten.then(f => f());
        }
    }, []);

    function isFiltering() {
        return filterText || filterOption.size != FILTER_TYPE_OPTIONS.length
    }

    function handleScroll(e) {
        if (!isFiltering()) {
            normalOffsetRef.current = e.target.scrollTop;
            localStorage.setItem(scrollStrogeId, e.target.scrollTop);
        }
    }

    let strucItems = [];
    compList.forEach((struc, name) => {
        if (filterText.length === 0 || filterText.indexOf(name) !== -1) {
            if (!filterOption.has("empty") && struc.key_paths.length === 0) {
                return
            }
            if (!filterOption.has("valid") && struc.key_paths.length) {
                return
            }
            if (!filterOption.has("single") && name.length === 1) {
                return
            }
            if (!filterOption.has("complex") && name.length !== 1) {
                return
            }
            strucItems.push({
                id: name,
                data: {
                    name,
                    struc,
                    markingOption,
                }
            })
        }
    })

    // Test
    // let name = "㐄";
    // strucItems = [{
    //     id: name,
    //     data: {
    //         name: name,
    //         markingOption: markingOption,
    //     }
    // }];

    return (
        <div style={{ display: "flex", flexDirection: "row", height: "100%" }}>
            <div style={{ display: "flex", flex: "1", flexDirection: "column" }}>
                <WorkspaceSettings
                    filterText={filterText}
                    setFilterText={setFilterText}
                    filterOption={filterOption}
                    setFilterOption={setFilterOption}
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