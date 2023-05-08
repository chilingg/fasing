import Footer from "./Footer";
import Settings from "./Settings";
import StrucDisplay from "./StrucDisplay";
import { ItemsScrollArea } from "@/widgets/Scroll";
import { Spacer } from "@/widgets/Space";
import { ActionBtn } from "@/widgets/Button";
import { SelectionLabel } from "@/widgets/Selection";
import { Horizontal, Vertical } from "@/widgets/Line";
import Input from "@/widgets/Input";

import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";

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

const ScrollStrogeId = "ComponentWorkspaceScroll";

export default function ComponentsWorkspace() {
    const [compNameList, setCompNameList] = useState([]);
    const [markingOption, setMarkingOption] = useState(new Set(MARK_OPTIONS));
    const [filterOption, setFilterOption] = useState(new Set(FILTER_TYPE_OPTIONS));
    const [filterText, setFilterText] = useState("");

    useEffect(() => {
        let unlisten = listen("source", (e) => {
            switch (e.payload.event) {
                case "load":
                    setCompNameList(e.payload.comp_names);
                    break;
            }
        });

        invoke("get_comp_name_list")
            .then(list => setCompNameList(list))

        return () => {
            unlisten.then(f => f());
        }
    }, []);

    function isFiltering() {
        return filterText || filterOption.size != FILTER_TYPE_OPTIONS.length
    }

    function handleMarkingOptionSet(e, active, value) {
        let newMarkingOp = new Set(markingOption);
        active ? newMarkingOp.add(value) : newMarkingOp.delete(value);
        setMarkingOption(newMarkingOp);
    }

    function handleFilterOptionSet(e, active, value) {
        let newOpiton = new Set(filterOption);
        active ? newOpiton.add(value) : newOpiton.delete(value);
        setFilterOption(newOpiton);
    }

    function handleScroll(e) {
        if (!isFiltering()) {
            localStorage.setItem(ScrollStrogeId, e.target.scrollTop);
        }
    }

    let strucItems = compNameList.filter(name => filterText.length === 0 || filterText.indexOf(name) !== -1)
        .map(name => {
            return {
                id: name,
                data: {
                    name: name,
                    markingOption: markingOption,
                }
            };
        });

    // Test
    // let name = "㐄";
    // strucItems = [{
    //     id: name,
    //     data: {
    //         name: name,
    //         markingOption: markingOption,
    //     }
    // }];

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

    return (
        <>
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
            <div style={{ flex: 1 }}>
                <ItemsScrollArea
                    ItemType={StrucDisplay}
                    items={strucItems}
                    onScroll={handleScroll}
                    offset={isFiltering() ? undefined : () => localStorage.getItem(ScrollStrogeId)}
                />
            </div>
            <Footer><Spacer /><p>部件 {compNameList.length}</p></Footer>
        </>
    );
}