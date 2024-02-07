import { MenuBtn } from "../widgets/Button"
import { List, Item } from "../widgets/List"
import Panel from "@/widgets/Panel"

export default function Header() {
    let list = [
        {
            text: "文件",
            items: [
                {
                    title: "重命名",
                    method: () => {
                        console.log("重命名")
                    }
                },
                {
                    title: "移动",
                    method: () => {
                        console.log("移动")
                    }
                }
            ]
        },
        {
            text: "帮助",
        }
    ];

    return (
        <Panel>
            <header style={{ display: "flex", flexDirection: "column" }}>
                <List direction="row">
                    {
                        list.map((item, index) =>
                            <Item key={index}><MenuBtn text={item.text} menuItems={item.items}></MenuBtn></Item>
                        )
                    }
                </List>
            </header>
        </Panel >
    )
}
