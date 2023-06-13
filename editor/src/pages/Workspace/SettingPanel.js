import Panel from "@/widgets/Panel";
import ResizableArea from "@/widgets/Resizable";
import Collapsible from "@/widgets/Collapsible";

export default function SettingPanel({ items, width, onResize }) {
    return (
        <ResizableArea style={{ display: "flex", flexDirection: "column", backgroundColor: "var(--panel-color)" }} left={true} onResize={onResize} width={width}>
            <div style={{ overflowY: "auto" }}>
                {items.map(item => (
                    <Collapsible key={item.id} title={item.title} open={item.open} setOpen={item.setOpen}>
                        {item.component}
                    </Collapsible>
                ))}
            </div>
        </ResizableArea>
    )
}