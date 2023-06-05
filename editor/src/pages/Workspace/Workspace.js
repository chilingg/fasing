import ComponentsWorkspace from "./ComponentWorkspace";
import CombinationWOrkspace from "./CombinationWorkspace";

import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";

export default function Workspace({ workStage }) {
    const [compList, setCompList] = useState(new Map());
    const [allocateTab, setAllocateTab] = useState([]);
    const [constructTab, setConstructTab] = useState(new Map());

    function loadData() {
        let strucInit = invoke("get_struc_proto_all");
        let allocateTabInit = invoke("get_allocate_table");

        Promise.all([strucInit, allocateTabInit])
            .then(([strucList, allocTab]) => {
                setCompList(new Map(Object.entries(strucList)))

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

        invoke("get_construct_table")
            .then(tab => setConstructTab(new Map(Object.entries(tab))));

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
            current = <CombinationWOrkspace constructTab={constructTab} />;
            break;
    }

    return (
        <div style={{ flex: 1 }}>
            {current}
        </div>
    )
}