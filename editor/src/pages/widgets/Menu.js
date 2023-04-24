import { List, Item } from "./List"

import { useState } from "react"

import style from "@/styles/Menu.module.css"

export default function Menu({ items, pos, close }) {
    if (pos) {
        return (
            <div className={style.menu} style={{ ...pos }}>
                <List direction="column">
                    {
                        items && items.map((item, index) => (
                            <Item key={index}>
                                <MenuItem text={item.text} action={item.action} close={close}></MenuItem>
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

function MenuItem({ text, action, close }) {
    switch (typeof action) {
        case "object":
            return <Items text={text} items={action} close={close}></Items>;
        case "function":
            return <button className={style.menuItem} onClick={() => { action(); close() }} onMouseDown={(e) => e.preventDefault()}>{text}</button>;
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
            <ItemsSymbol />
            <Menu items={items} pos={visible} close={close}></Menu>
        </div >
    )
}

function ItemsSymbol() {
    return (
        <svg style={{ float: "right", width: 12, height: 12 }}>
            <polyline points="6,2 10,6 6,10"></polyline>
        </svg>
    )
}