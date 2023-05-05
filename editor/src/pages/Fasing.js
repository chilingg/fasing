import Nav from "./Nav";
import Workspace from "./Workspace/Workspace";
import SettingPanel from "./SettingPanel";

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

import style from "@/styles/Fasing.module.css";

export default function Fasing() {
    const [workStage, setWorkStage] = useState();

    function setWorkStageAndStorage(stage) {
        setWorkStage(stage);
        invoke("set_context_value", { key: "workStage", value: stage });
    }

    useEffect(() => {
        let unlisten = listen("source", (e) => {
            if (e.payload.event == "none") {
                console.error("Undefine source none event!");
            }
        });
        invoke("get_context_value", { key: "workStage" })
            .then(stage => {
                if (stage && stage !== workStage) {
                    setWorkStage(stage);
                }
            })

        let intervalId = setInterval(() => {
            invoke("save_context").catch(() => console.error("Failed to save context!"));
        }, 1000 * 60 * 20);

        return () => {
            clearInterval(intervalId);
            unlisten.then(f => f());
        }
    }, []);

    return (
        <main className={style.fasing} >
            <Nav workStage={workStage} setWorkStage={setWorkStageAndStorage}></Nav>
            <Workspace workStage={workStage}></Workspace>
            <SettingPanel workStage={workStage} />
        </main >
    )
}