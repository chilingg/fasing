import Panel from "@/widgets/Panel";
import ResizableArea from "@/widgets/Area";
import Collapsible from "@/widgets/Collapsible";

import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState } from "react";

export default function SettingsPanel() {
    const [width, setWidth] = useState(360);
    const [openSelecte, setOpenSelecte] = useState(false)
    const [openAllocate, setOpenAllocate] = useState(true)
    const [openRegexTest, setOpenRegexTest] = useState(true)

    useEffect(() => {
        invoke("get_context_value", { key: ["settingPanel", "width"] })
            .then(width => setWidth(width))
    }, []);

    function handleResize(rect) {
        invoke("set_context_value", { key: ["settingPanel", "width"], value: rect.width });
    }

    return (
        <ResizableArea style={{ display: "flex", flexDirection: "column" }} left={true} onResize={handleResize} width={width}>
            <Collapsible title={"选中"} open={openSelecte} setOpen={setOpenSelecte}>选中</Collapsible>
            <Collapsible title={"空间分配"} open={openAllocate} setOpen={setOpenAllocate}>分配</Collapsible>
            <Collapsible title={"规则测试"} open={openRegexTest} setOpen={setOpenRegexTest}>测试</Collapsible>
            <Panel style={{ flex: "1 1 auto" }} />
        </ResizableArea>
    )
}