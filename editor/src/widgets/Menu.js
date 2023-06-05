import { List, Item } from "./List"
import { shortcutText } from "@/lib/actions"
import { GreaterThanIcon } from "./Icons"
import { useRef, useState, useEffect } from "react"

import style from "@/styles/Menu.module.css"

export default function Menu({ items, pos, close }) {
    if (pos) {
        return (
            <div className={style.menu} style={{ ...pos }}>
                <List direction="column">
                    {
                        items && items.map((item, index) => (
                            <Item key={index}>
                                <MenuItem {...item} close={close}></MenuItem>
                            </Item>
                        ))
                    }
                </List>
            </div >
        )
    } else {
        return null
    }
}

export function ContentPanel({ pos, setClose, children, ...props }) {
    const state = useRef(false);

    function blur() {
        if (state.current) {
            state.current = false;
        } else {
            setClose();
        }
    }

    useEffect(() => {
        if (pos) {
            window.addEventListener("mousedown", blur);
            return () => window.removeEventListener("mousedown", blur);
        }
    }, [pos]);

    if (pos) {
        return (
            <div
                className={style.contentPanel}
                style={{ ...pos }}
                onMouseMove={e => e.stopPropagation()}
                onClick={e => e.stopPropagation()}
                onMouseDown={() => {
                    state.current = true;
                }}
                {...props}
            >
                {children}
            </div >
        )
    } else {
        return null
    }
}

export function Tips({ tips, children }) {
    const [pos, setPos] = useState(null);
    const ref = useRef();
    return (
        <div ref={ref} className={style.positionter} onMouseLeave={() => setPos(null)} onMouseEnter={e => {
            let rect = ref.current.offsetParent.getBoundingClientRect();
            setPos({ left: e.clientX - rect.x, top: e.clientY - rect.y });
        }} >
            {children}
            <ContentPanel pos={pos} setClose={e => setPos(null)}>
                <p>{tips}</p>
            </ContentPanel>
        </div>
    )
}

function MenuItem({ text, action, close, shortcut }) {
    switch (typeof action) {
        case "object":
            return <Items text={text} items={action} close={close}></Items>;
        case "function":
            return (
                <button className={style.menuItem} onClick={() => { action(); close() }} onMouseDown={(e) => e.preventDefault()}>
                    {text}
                    {shortcut && <span className={style.shortcut}>{shortcutText(shortcut)}</span>}
                </button>
            );
        default:
            return <hr></hr>
    }
}

function Items({ text, items, close }) {
    const [visible, setVisible] = useState(null);

    function onMouseOver(e) {
        let rect = e.currentTarget.getBoundingClientRect();
        let pos = { left: rect.right }

        if (window.innerHeight - rect.bottom < rect.top) {
            pos.bottom = window.innerHeight - rect.bottom;
        } else {
            pos.top = rect.top;
        }

        setVisible(pos);
    }

    function onMouseOut() {
        setVisible(null);
    }

    return (
        <div className={style.menuItems} onMouseEnter={onMouseOver} onMouseLeave={onMouseOut}>
            {text}
            <GreaterThanIcon style={{ float: "right", width: 12, height: 12 }} />
            <Menu items={items} pos={visible} close={close}></Menu>
        </div >
    )
}
