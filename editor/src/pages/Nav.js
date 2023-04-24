import style from "@/styles/Nav.module.css"
import { List, Item } from "./widgets/List"
import { IconBtn } from "./widgets/Button"
import Separator from "./widgets/Separator"
import Panel from "./Panel"
import Menu from "./widgets/Menu"

import { useState } from "react"

let btnStyle = { width: "100%" };
let iconSize = { width: 32, height: 56 };
let offsetY = (iconSize.height - iconSize.width) * 0.5 + 4;

export default function Nav() {
    return (
        <Panel>
            <nav className={style.nav}>
                <List direction="column">
                    <Item>
                        <IconBtn btnStyle={btnStyle}>
                            <svg style={iconSize}>
                                <rect x="4" y={4 + offsetY} width="6" height="6"></rect>
                                <line x1="10" y1={7 + offsetY} x2="22" y2={7 + offsetY} />
                                <rect x="22" y={4 + offsetY} width="6" height="6"></rect>
                                <line x1="22" y1={10 + offsetY} x2="10" y2={24 + offsetY} />
                                <rect x="4" y={24 + offsetY} width="6" height="6"></rect>
                            </svg>
                        </IconBtn>
                    </Item>
                    <Item>
                        <IconBtn btnStyle={btnStyle}>
                            <svg style={iconSize}>
                                <rect x="5" y={4 + offsetY} width="8" height="24"></rect>
                                <rect x="19" y={4 + offsetY} width="8" height="8"></rect>
                                <rect x="19" y={18 + offsetY} width="8" height="10"></rect>
                            </svg>
                        </IconBtn>
                    </Item>
                    <Item>
                        <IconBtn btnStyle={btnStyle}>
                            <svg style={iconSize}>
                                <polyline points={`8,${28 + offsetY} 8,${4 + offsetY} 24,${4 + offsetY} 24,${28 + offsetY} `}></polyline>
                                <line x1="16" y1={4 + offsetY} x2="16" y2={28 + offsetY} />
                                <line x1="2" y1={16 + offsetY} x2="30" y2={16 + offsetY} />
                            </svg>
                        </IconBtn>
                    </Item>
                </List>
                <Separator></Separator>
                <List direction="column">
                    <Item>
                        <MenuIcon></MenuIcon>
                    </Item>
                </List>
            </nav>
        </Panel>
    )
}

function MenuIcon() {
    let list = [
        {
            text: "文件",
            action: [
                {
                    text: "打开",
                    action: () => {
                        window.__TAURI__.dialog.open().then(file => console.log(file));
                    },
                },
                {
                    text: "保存",
                    action: () => {
                        console.log("移动")
                    }
                },
                {
                    text: "另存为",
                    action: () => {
                        window.__TAURI__.dialog.save().then(file => console.log(file));
                    }
                }
            ]
        },
        {},
        {
            text: "帮助",
            action: () => console.log("帮助"),
        }
    ];
    const [menuPos, setMenuPos] = useState(null);

    function handleClick(e) {
        let btn = e.currentTarget;
        setMenuPos({ left: btn.offsetLeft + btn.offsetWidth, bottom: window.innerHeight - btn.offsetHeight - btn.offsetTop });
    }

    function handleBlur() {
        setMenuPos(null);
    }

    return (
        <div onBlur={handleBlur}>
            <IconBtn btnStyle={btnStyle} onClick={handleClick} active={menuPos}>
                <svg style={iconSize}>
                    <line x1="4" y1={6 + offsetY} x2="28" y2={6 + offsetY} />
                    <line x1="4" y1={16 + offsetY} x2="28" y2={16 + offsetY} />
                    <line x1="4" y1={26 + offsetY} x2="28" y2={26 + offsetY} />
                </svg>
            </IconBtn>
            <Menu items={list} pos={menuPos} close={() => setMenuPos(null)}></Menu>
        </div>
    )
}