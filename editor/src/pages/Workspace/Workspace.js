import ComponentsWorkspace from "./ComponentWorkspace";
import CombinationWorkspace from "./CombinationWorkspace";

import { useState, useEffect } from "react";
import { useImmer } from "use-immer";
import { enableMapSet } from "immer"
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";

enableMapSet();

export default function Workspace({ workStage }) {
    const [constructTab, setConstructTab] = useState(new Map());
    const [compList, updateCompList] = useImmer(new Map());

    useEffect(() => {
        invoke("get_construct_table")
            .then(tab => setConstructTab(new Map(Object.entries(tab))));
        invoke("get_struc_proto_all").then(list => updateCompList(draft => draft = new Map(Object.entries(list))));

        let unlistenStrucChange = listen("struc_change", (e) => {
            let name = e.payload;
            invoke("get_struc_proto", { name })
                .then(struc => updateCompList(draft => draft.set(name, struc)));
        });

        return () => unlistenStrucChange.then(f => f());
    }, []);

    let current = <div></div>;

    switch (workStage) {
        case "components":
            current = <ComponentsWorkspace compList={compList} />;
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