import "./App.css";
import Middle from './widget/middle/Middle';
import Right from './widget/right/Right';
import Left from './widget/left/Left';
import MainMenu from './widget/MainMenu';
import { TYPE_FILTERS } from './lib/construct';
import { Context, STORAGE_ID } from "./lib/storageld";

import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useState, useEffect } from "react";
import { useImmer } from 'use-immer';
import { Splitter } from 'antd';

import { theme } from 'antd';
const { useToken } = theme;

const DEFAULT_CHAR_DISPLAY = {
    background: '#ffffff',
    color: '#000000',
    size: 48,
    charName: true,
}

const App = () => {
    const { token } = useToken();
    const [cstTable, setCstTable] = useState({})
    const [targetChars, setTargetChars] = useState([]);
    const [config, updateConfigProto] = useImmer();
    const [selectedChar, setSelectedChar] = useState();

    const [charFilter, setCharFilterProto] = useState(() => {
        const value = Context.getItem(STORAGE_ID.left.filter);
        return value || { text: "", types: [] }
    });
    const [charDisplay, setCharDisplayProto] = useState(() => {
        const value = Context.getItem(STORAGE_ID.left.charDisplay);
        return value || DEFAULT_CHAR_DISPLAY
    });

    useEffect(() => {
        function update() {
            invoke("target_chars", {}).then(list => setTargetChars(list));
            invoke("get_cst_table", {}).then(table => setCstTable(table));
            invoke("get_config", {}).then(config => updateConfigProto(draft => draft = config));
        }

        update();

        let unlistenStrucChange = listen("source", (e) => {
            update()
        });

        return () => unlistenStrucChange.then(f => f());
    }, []);

    function updateConfig(f) {
        let newCfg = JSON.parse(JSON.stringify(config));
        f(newCfg);
        invoke("set_config", { cfg: newCfg });
        updateConfigProto(draft => draft = newCfg);
    }

    function setCharFilter(filter) {
        setCharFilterProto(filter)
        Context.setItem(STORAGE_ID.left.filter, filter)
    }

    function setCharDisplay(value) {
        Context.setItem(STORAGE_ID.left.charDisplay, value);
        setCharDisplayProto(value);
    }

    function handleSideResize(left, right) {
        if (left !== leftDefaultSize) {
            Context.setItem(STORAGE_ID.left.width, left);
        }
        if (right !== rightDefaultSize) {
            Context.setItem(STORAGE_ID.right.width, right);
        }
    }

    let leftDefaultSize = Context.getItem(STORAGE_ID.left.width);
    let rightDefaultSize = Context.getItem(STORAGE_ID.right.width);

    let charList;
    if (charFilter.text.length) {
        let temp = charFilter.text.split(' ');
        charList = [...temp[0], ...temp.slice(1)];
    } else {
        charList = targetChars;
    }
    if (charFilter.types.length) {
        let filters = charFilter.types.map(tp => TYPE_FILTERS.get(tp));
        charList = charList.filter(char => {
            let attrs = cstTable[char];
            console.log(char, attrs)
            return ((attrs || attrs == "") && (filters.find(f => f(attrs))) || attrs == undefined)
        })
    }
    // charList = ['口']
    let sideBarStyle = { boxShadow: token.boxShadow, padding: token.containerPadding, backgroundColor: token.colorBgElevated };

    return <>
        <Splitter
            style={{ height: '100vh', backgroundColor: token.colorBgBase }}
            onResizeEnd={([left, middle, right]) => handleSideResize(left, right)}
        >
            <Splitter.Panel
                style={sideBarStyle}
                defaultSize={leftDefaultSize ? leftDefaultSize : 320}
            >
                <Left
                    charDisplay={charDisplay} setCharDisplay={setCharDisplay}
                    charFilter={charFilter} setCharFilter={setCharFilter}
                    strokWidth={config?.strok_width}
                />
            </Splitter.Panel>

            <Splitter.Panel>
                <Middle
                    charList={charList}
                    charDisplay={charDisplay}
                    strokeWidth={config?.strok_width}
                    selectedChar={selectedChar}
                    setSelectedChar={setSelectedChar}
                />
            </Splitter.Panel>

            <Splitter.Panel
                style={sideBarStyle}
                defaultSize={rightDefaultSize ? rightDefaultSize : 300}
            >
                <Right config={config} updateConfig={updateConfig} selectedChar={selectedChar}></Right>
            </Splitter.Panel>
        </Splitter>

        <MainMenu />
    </>
};
export default App;