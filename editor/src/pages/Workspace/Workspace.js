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

    useEffect(() => {
        invoke("get_construct_table")
            .then(tab => setConstructTab(new Map(Object.entries(tab))));

        return () => unlistenStrucChange.then(f => f());
    }, []);

    let current = <div></div>;

    switch (workStage) {
        case "components":
            current = <ComponentsWorkspace />;
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