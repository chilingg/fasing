import Panel from "./Panel"
import { GreaterThanIcon } from "./Icons"
import ResizableArea from "./Resizable";
import { SubPanel } from "./Panel";

import style from "@/styles/Collapsible.module.css"

export default function Collapsible({ children, open, setOpen, title }) {
    return (
        <div>
            <Panel className={style.head} onClick={() => setOpen(!open)}>
                <GreaterThanIcon className={style.icon} size={14} opened={open ? "" : undefined} />
                <h1>{title}</h1>
            </Panel>
            {open && (
                <ResizableArea bottom={true}>
                    <SubPanel style={{ height: "100%" }} >
                        {children}
                    </SubPanel>
                </ResizableArea>
            )}
        </div>
    )
}