import Nav from "./Nav";
import Workspace from "./Workspace/Workspace";

import { useState, useEffect } from "react";

import { STORAGE_ID, Context } from "@/lib/storageId";
import style from "@/styles/Fasing.module.css";

export default function Fasing() {
    const [workStage, setWorkStage] = useState();

    useEffect(() => {
        setWorkStage(Context.getItem(STORAGE_ID.workStage));
    }, [])

    function setWorkStageAndStorage(stage) {
        setWorkStage(stage);
        Context.setItem(STORAGE_ID.workStage, stage);
    }

    return (
        <main className={style.fasing} >
            <Nav workStage={workStage} setWorkStage={setWorkStageAndStorage}></Nav>
            <Workspace workStage={workStage}></Workspace>
        </main >
    )
}