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
            invoke("target_chars", {}).then(list => setTargetChars(list))
            invoke("get_cst_table", {}).then(table => setCstTable(table))
        }

        update();

        let unlistenStrucChange = listen("source", (e) => {
            update()
        });

        return () => unlistenStrucChange.then(f => f());
    }, []);

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

    let charList = charFilter.text.length ? [...charFilter.text] : targetChars;
    if (charFilter.types.length) {
        let filters = charFilter.types.map(tp => TYPE_FILTERS.get(tp));
        charList = charList.filter(char => {
            let attrs = cstTable[char];
            return (attrs || attrs == "") && (filters.find(f => f(attrs)))
        })
    }
    // charList = ['口', '仁']
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
                />
            </Splitter.Panel>

            <Splitter.Panel>
                <Middle charList={charList} charDisplay={charDisplay} />
            </Splitter.Panel>

            <Splitter.Panel
                style={sideBarStyle}
                defaultSize={rightDefaultSize ? rightDefaultSize : 300}
            >
                <Right></Right>
            </Splitter.Panel>
        </Splitter>

        <MainMenu />
    </>
};
export default App;