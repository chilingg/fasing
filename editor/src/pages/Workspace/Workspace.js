import ComponentsWorkspace from "./ComponentWorkspace";
import CombinationWOrkspace from "./CombinationWorkspace";

import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";
import { STORAGE_ID, Context } from "@/lib/storageId";

export default function Workspace({ workStage }) {
    const [compList, setCompList] = useState(new Map());
    const [allocateTab, setAllocateTab] = useState([]);

    function loadData() {
        let strucInit = invoke("get_struc_proto_all");
        let allocateTabInit = invoke("get_allocate_table");

        Promise.all([strucInit, allocateTabInit])
            .then(([strucList, allocTab]) => {
                setCompList(new Map(Object.entries(strucList)))

                let colors = Context.getItem(STORAGE_ID.allocateTable.colors);
                if (!colors || allocTab.length !== colors.length) {
                    colors = Array(allocTab.length).fill(0);
                }
                for (let i = 0; i < allocTab.length; ++i) {
                    allocTab[i].color = colors[i];
                }

                allocTab.forEach(rule => rule.regex = new RegExp(rule.regex));
                setAllocateTab(allocTab);
            })
    }

    useEffect(() => {
        loadData();

        let unlisten = listen("source", (e) => {
            switch (e.payload.event) {
                case "load":
                    loadData();
                    break;
            }
        });

        return () => {
            unlisten.then(f => f());
        }
    }, []);

    let current = <div></div>;

    switch (workStage) {
        case "components":
            current = <ComponentsWorkspace compList={compList} setCompList={setCompList} allocateTab={allocateTab} setAllocateTab={setAllocateTab} />;
            break;
        case "combination":
            current = <CombinationWOrkspace />;
            break;
    }

    return (
        <div style={{ flex: 1 }}>
            {current}
        </div>
    )
}