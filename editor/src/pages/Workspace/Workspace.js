import ComponentsWorkspace from "./ComponentWorkspace";
import CombinationWorkspace from "./CombinationWorkspace";

import { useState, useEffect } from "react";
import { useImmer } from "use-immer";
import { enableMapSet } from "immer"
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";

enableMapSet();

export default function Workspace({ workStage }) {
    const [compList, updateCompList] = useImmer(new Map());
    const [allocateTab, setAllocateTab] = useState([]);
    const [constructTab, setConstructTab] = useState(new Map());

    function loadData() {
        let strucInit = invoke("get_struc_proto_all");
        let allocateTabInit = invoke("get_allocate_table");

        Promise.all([strucInit, allocateTabInit])
            .then(([strucList, allocTab]) => {
                updateCompList(draft => draft = new Map(Object.entries(strucList)));

                allocTab.forEach(rule => rule.regex = new RegExp(rule.regex));
                setAllocateTab(allocTab);
            })
            .catch(e => console.error(e))
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
        let unlistenStrucChange = listen("struc_change", (e) => {
            let name = e.payload;
            invoke("get_struc_proto", { name })
                .then(struc => updateCompList(draft => draft.set(name, struc)));
        })

        invoke("get_construct_table")
            .then(tab => setConstructTab(new Map(Object.entries(tab))));

        return () => {
            unlisten.then(f => f());
            unlistenStrucChange.then(f => f());
        }
    }, []);

    let current = <div></div>;

    switch (workStage) {
        case "components":
            current = <ComponentsWorkspace compList={compList} allocateTab={allocateTab} setAllocateTab={setAllocateTab} />;
            break;
        case "combination":
            current = <CombinationWorkspace constructTab={constructTab} />;
            break;
    }

    return (
        <div style={{ flex: 1 }}>
            {current}
        </div>
    )
}