import Panel from "./Panel"
import { GreaterThanIcon } from "./Icons"
import ResizableArea from "./Resizable";
import { SubPanel } from "./Panel";

import { useState } from "react";

import { Context } from "@/lib/storageId";
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

export function SimpleCollapsible({ children, title, onAction, storageId = null, defaultOpen = true }) {
    let storage = storageId ? Context.getItem(storageId) : null;
    const [open, setOpenProto] = useState(storage === null ? defaultOpen : storage);

    function setOpen(open) {
        setOpenProto(open);
        storageId && Context.setItem(storageId, open);
    }

    return (
        <div>
            <div
                onClick={e => setOpen(onAction ? onAction(!open) : !open)}
                onMouseDown={e => e.preventDefault()}
            >
                <GreaterThanIcon className={style.icon} size={16} opened={open ? "" : undefined} />
                {title}
            </div>
            {open && <div style={{ paddingLeft: "1em" }}>{children}</div>}
        </div>
    )
}