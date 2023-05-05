import Panel from "@/widgets/Panel";
import ResizableArea from "@/widgets/Area";

import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState } from "react";

export default function SettingsPanel() {
    const [width, setWidth] = useState(360);

    useEffect(() => {
        invoke("get_context_value", { key: ["settingPanel", "width"] })
            .then(width => setWidth(width))
    }, []);

    function handleResize(rect) {
        invoke("set_context_value", { key: ["settingPanel", "width"], value: rect.width });
    }

    return (
        <ResizableArea left={true} onResize={handleResize} width={width}>
            <Panel style={{ width: "100%", height: "100%" }}>
                <p>------------------------侧边栏------------------------</p>
            </Panel>
        </ResizableArea>
    )
}