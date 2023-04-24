import Nav from "./Nav"
import Workspace from "./Workspace/Workspace"
import SettingsArea from "./SettingArea"

import style from "@/styles/Fasing.module.css"

export default function Fasing() {
    return (
        <main className={style.fasing}>
            <Nav></Nav>
            <Workspace></Workspace>
            <SettingsArea></SettingsArea>
        </main >
    )
}