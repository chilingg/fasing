import Panel from "@/widgets/Panel";
import ResizableArea from "@/widgets/Area";
import Collapsible from "@/widgets/Collapsible";

export default function SettingsPanel({ items, width, onResize }) {
    return (
        <ResizableArea style={{ display: "flex", flexDirection: "column" }} left={true} onResize={onResize} width={width}>
            {items.map(item => (
                <Collapsible key={item.id} title={item.title} open={item.open} setOpen={item.setOpen}>
                    {item.component}
                </Collapsible>
            ))}
            <Panel style={{ flex: "1 1 auto" }} />
        </ResizableArea>
    )
}