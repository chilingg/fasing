import Settings from "./Settings";
import { Horizontal, Vertical } from "@/widgets/Line";

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

export default function CombinationWOrkspace({ }) {
    return (
        <div style={{ display: "flex", flexDirection: "row", height: "100%" }}>
            {/* <div style={{ display: "flex", flex: "1", flexDirection: "column" }}>
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
            <WorkspaceSettingPanel allocateTab={allocateTab} setAllocateTab={setAllocateTab} /> */}
        </div>
    )
}