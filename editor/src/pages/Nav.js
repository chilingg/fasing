import { List, Item } from "@/widgets/List"
import { IconBtn } from "@/widgets/Button"
import { Spacer } from "@/widgets/Space"
import Panel from "@/widgets/Panel"
import Menu from "@/widgets/Menu"
import { SHORTCUT, isKeydown } from "@/lib/actions"

import { useState, useEffect } from "react"
import { invoke } from "@tauri-apps/api/tauri"
import * as dialog from "@tauri-apps/api/dialog"

import style from "@/styles/Nav.module.css"

let btnStyle = { width: "100%" };
let iconSize = { width: 32, height: 56 };
let offsetY = (iconSize.height - iconSize.width) * 0.5 + 4;

let menu_list = [
    {
        text: "文件",
        action: [
            {
                text: "打开",
                action: () => {
                    dialog.open().then(file => {
                        if (file) {
                            invoke("new_service_from_file", { path: file })
                                .catch((e) => console.error(e))
                        }
                    });

                },
                shortcut: SHORTCUT["open"]
            },
            {
                text: "保存",
                action: () => {
                    invoke("save_service_file")
                },
                shortcut: SHORTCUT["save"]
            },
            {
                text: "另存为",
                action: () => {
                    console.log("另存为")
                    dialog.save().then(file => console.log(file));
                },
                shortcut: SHORTCUT.save_as
            },
            {
                text: "重载",
                action: () => invoke("reload"),
                shortcut: SHORTCUT.reload
            }
        ]
    },
    {},
    {
        text: "帮助",
        action: () => console.log("帮助"),
    }
];

const stageIcons = [
    {
        stage: "characters", tip: "字符集", icon: (
            <svg style={iconSize}>
                <polyline points={`8,${28 + offsetY} 8,${4 + offsetY} 24,${4 + offsetY} 24,${28 + offsetY} `}></polyline>
                <line x1="16" y1={4 + offsetY} x2="16" y2={28 + offsetY} />
                <line x1="2" y1={16 + offsetY} x2="30" y2={16 + offsetY} />
            </svg>
        )
    },
    {
        stage: "components", tip: "部件", icon: (
            <svg style={iconSize}>
                <rect x="4" y={4 + offsetY} width="6" height="6"></rect>
                <line x1="10" y1={7 + offsetY} x2="22" y2={7 + offsetY} />
                <rect x="22" y={4 + offsetY} width="6" height="6"></rect>
                <line x1="22" y1={10 + offsetY} x2="10" y2={24 + offsetY} />
                <rect x="4" y={24 + offsetY} width="6" height="6"></rect>
            </svg>
        )
    },
    {
        stage: "combination", tip: "组合", icon: (
            <svg style={iconSize}>
                <rect x="5" y={4 + offsetY} width="8" height="24"></rect>
                <rect x="19" y={4 + offsetY} width="8" height="8"></rect>
                <rect x="19" y={18 + offsetY} width="8" height="10"></rect>
            </svg>
        )
    },
]

function MenuIcon() {
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
            <Menu items={menu_list} pos={menuPos} close={() => setMenuPos(null)}></Menu>
        </div>
    )
}

export default function Nav({ workStage, setWorkStage }) {
    function menuKeyDown(e, menuItem) {
        if (menuItem && menuItem.action) {
            switch (typeof menuItem.action) {
                case "object":
                    for (let item in menuItem.action) {
                        menuKeyDown(e, menuItem.action[item]);
                    }
                    break;
                case "function":
                    if (menuItem.shortcut && isKeydown(e, menuItem.shortcut)) {
                        menuItem.action();
                    }
            }
        }
    }

    function handleKeyDown(e) {
        for (let i = 0; i < menu_list.length; ++i) {
            menuKeyDown(e, menu_list[i]);
        }
    }

    useEffect(() => {
        window.addEventListener("keydown", handleKeyDown);
        return () => window.removeEventListener("keydown", handleKeyDown);
    }, []);

    let icons = stageIcons.map(icon => {
        return (
            <Item key={icon.stage}>
                <IconBtn btnStyle={btnStyle} active={icon.stage === workStage} onClick={() => setWorkStage(icon.stage)}>
                    {icon.icon}
                </IconBtn>
            </Item>
        )
    });

    return (
        <Panel>
            <nav className={style.nav}>
                <List direction="column">
                    {icons}
                </List>
                <Spacer />
                <List direction="column">
                    <Item>
                        <MenuIcon />
                    </Item>
                </List>
            </nav>
        </Panel>
    )
}
