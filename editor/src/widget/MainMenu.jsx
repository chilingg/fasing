import { SHORTCUT, shortcutText, isKeydown } from '../lib/action';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import { FloatButton, Menu, theme } from 'antd';
import { useEffect, useState } from 'react';

const { useToken } = theme;

const ShortcutText = (props) => {
    const { token } = useToken();

    return <>
        {props.children}
        <span style={{ color: token.colorTextQuaternary }}> {shortcutText(props.shortcut)}</span>
    </>
}

const menuItems = [
    {
        key: 'save',
        label: <ShortcutText shortcut={SHORTCUT.save}>保存</ShortcutText>,
        icon: <SvaeIcon />,
        shortcut: SHORTCUT.save,
        onClick: () => {
            invoke("save_fas_file", {}).catch(e => console.error(e));
        }
    },
    {
        key: 'reload',
        label: <ShortcutText shortcut={SHORTCUT.reload}>重载</ShortcutText>,
        icon: <ReloadIcon />,
        shortcut: SHORTCUT.reload,
        onClick: () => {
            invoke("reload", {}).catch(e => console.error(e));
        }
    },
    {
        key: 'open',
        label: <ShortcutText shortcut={SHORTCUT.open}>打开</ShortcutText>,
        icon: <FileIcon />,
        shortcut: SHORTCUT.open,
        onClick: () => {
            open({
                multiple: false,
                directory: false,
            }).then(file => {
                if (file) {
                    invoke("new_source", { path: file }).catch(e => console.error(e));
                }
            })
        }
    },
    {
        key: 'export',
        label: <ShortcutText shortcut={SHORTCUT.export}>导出</ShortcutText>,
        icon: <FileIcon />,
        shortcut: SHORTCUT.export,
        onClick: () => {
            open({
                multiple: false,
                directory: true,
            }).then(dir => {
                if (dir) {
                    invoke("export_chars", { list: [], width: 1000, height: 1000, path: dir })
                        .then(message => {
                            for (let i = 0; i < 10 & i < message.length; ++i) {
                                console.error(message[i]);
                            }
                            console.log(`Export completed, with ${message.length} errors occurred.`)
                        })
                        .catch(e => console.error(e));
                }
            })
        }
    },
];

const MainMenu = () => {
    const [changed, setChanged] = useState(false);

    function handleKeyDown(e) {
        for (let i = 0; i < menuItems.length; ++i) {
            if (isKeydown(e, menuItems[i].shortcut)) {
                menuItems[i].onClick();
            }
        }
    }

    useEffect(() => {
        invoke("is_changed", {}).then(b => {
            b !== changed && setChanged(!changed)
        });

        window.addEventListener("keydown", handleKeyDown);

        let unlistenChanged = listen("changed", (e) => {
            setChanged(true);
        });
        let unlistenSaved = listen("saved", (e) => {
            setChanged(false);
        });
        let unlistenSource = listen("source", (e) => {
            invoke("is_changed", {}).then(b => setChanged(b));
        });


        return () => {
            window.removeEventListener("keydown", handleKeyDown);
            unlistenChanged.then(f => f());
            unlistenSaved.then(f => f());
            unlistenSource.then(f => f());
        }
    }, []);

    return <>
        <FloatButton.Group
            trigger="hover"
            type={changed ? "primary" : "default"}
            style={{ insetInlineEnd: undefined }}
            icon={<MenuIcon />}
        >
            <Menu style={{ width: 256 }} mode="vertical" items={menuItems} />
        </FloatButton.Group>

    </>
}

export default MainMenu;

export function MenuIcon(props) {
    return (
        <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 1024 1024" {...props}><path fill="currentColor" d="M912 192H328c-4.4 0-8 3.6-8 8v56c0 4.4 3.6 8 8 8h584c4.4 0 8-3.6 8-8v-56c0-4.4-3.6-8-8-8m0 284H328c-4.4 0-8 3.6-8 8v56c0 4.4 3.6 8 8 8h584c4.4 0 8-3.6 8-8v-56c0-4.4-3.6-8-8-8m0 284H328c-4.4 0-8 3.6-8 8v56c0 4.4 3.6 8 8 8h584c4.4 0 8-3.6 8-8v-56c0-4.4-3.6-8-8-8M104 228a56 56 0 1 0 112 0a56 56 0 1 0-112 0m0 284a56 56 0 1 0 112 0a56 56 0 1 0-112 0m0 284a56 56 0 1 0 112 0a56 56 0 1 0-112 0" /></svg>
    )
}

export function FileIcon(props) {
    return (
        <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 1024 1024" {...props}><path fill="currentColor" d="M854.6 288.6L639.4 73.4c-6-6-14.1-9.4-22.6-9.4H192c-17.7 0-32 14.3-32 32v832c0 17.7 14.3 32 32 32h640c17.7 0 32-14.3 32-32V311.3c0-8.5-3.4-16.7-9.4-22.7M790.2 326H602V137.8zm1.8 562H232V136h302v216a42 42 0 0 0 42 42h216zM504 618H320c-4.4 0-8 3.6-8 8v48c0 4.4 3.6 8 8 8h184c4.4 0 8-3.6 8-8v-48c0-4.4-3.6-8-8-8M312 490v48c0 4.4 3.6 8 8 8h384c4.4 0 8-3.6 8-8v-48c0-4.4-3.6-8-8-8H320c-4.4 0-8 3.6-8 8" /></svg>
    )
}

export function ReloadIcon(props) {
    return (
        <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 1024 1024" {...props}><path fill="currentColor" d="m909.1 209.3l-56.4 44.1C775.8 155.1 656.2 92 521.9 92C290 92 102.3 279.5 102 511.5C101.7 743.7 289.8 932 521.9 932c181.3 0 335.8-115 394.6-276.1c1.5-4.2-.7-8.9-4.9-10.3l-56.7-19.5a8 8 0 0 0-10.1 4.8c-1.8 5-3.8 10-5.9 14.9c-17.3 41-42.1 77.8-73.7 109.4A344.8 344.8 0 0 1 655.9 829c-42.3 17.9-87.4 27-133.8 27c-46.5 0-91.5-9.1-133.8-27A341.5 341.5 0 0 1 279 755.2a342.2 342.2 0 0 1-73.7-109.4c-17.9-42.4-27-87.4-27-133.9s9.1-91.5 27-133.9c17.3-41 42.1-77.8 73.7-109.4s68.4-56.4 109.3-73.8c42.3-17.9 87.4-27 133.8-27c46.5 0 91.5 9.1 133.8 27a341.5 341.5 0 0 1 109.3 73.8c9.9 9.9 19.2 20.4 27.8 31.4l-60.2 47a8 8 0 0 0 3 14.1l175.6 43c5 1.2 9.9-2.6 9.9-7.7l.8-180.9c-.1-6.6-7.8-10.3-13-6.2" /></svg>
    )
}

export function SvaeIcon(props) {
    return (
        <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 1024 1024" {...props}><path fill="currentColor" d="M893.3 293.3L730.7 130.7c-7.5-7.5-16.7-13-26.7-16V112H144c-17.7 0-32 14.3-32 32v736c0 17.7 14.3 32 32 32h736c17.7 0 32-14.3 32-32V338.5c0-17-6.7-33.2-18.7-45.2M384 184h256v104H384zm456 656H184V184h136v136c0 17.7 14.3 32 32 32h320c17.7 0 32-14.3 32-32V205.8l136 136zM512 442c-79.5 0-144 64.5-144 144s64.5 144 144 144s144-64.5 144-144s-64.5-144-144-144m0 224c-44.2 0-80-35.8-80-80s35.8-80 80-80s80 35.8 80 80s-35.8 80-80 80" /></svg>
    )
}
