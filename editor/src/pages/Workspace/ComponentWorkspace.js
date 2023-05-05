import Footer from "./Footer";
import Settings from "./Settings";
import StrucDisplay from "./StrucDisplay";
import { ItemsScrollArea } from "@/widgets/Scroll";
import Separator from "@/widgets/Separator";

import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";

export default function ComponentsWorkspace() {
    const [compNameList, setCompNameList] = useState([]);

    useEffect(() => {
        let unlisten = listen("source", (e) => {
            switch (e.payload.event) {
                case "load":
                    setCompNameList(e.payload.comp_names);
                    break;
            }
        });

        invoke("get_comp_name_list")
            .then(list => setCompNameList(list))

        return () => {
            unlisten.then(f => f());
        }
    }, []);

    let strucItems = compNameList.map(name => {
        return {
            id: name,
            data: {
                name: name
            }
        };
    });

    function handleScroll(e) {
        console.log(e.target.scrollTop);
    }

    return (
        <>
            <Settings></Settings>
            <div style={{ flex: 1 }}>
                <ItemsScrollArea ItemType={StrucDisplay} items={strucItems} onScroll={handleScroll} />
            </div>
            <Footer><Separator /><p>部件 {compNameList.length}</p></Footer>
        </>
    );
}