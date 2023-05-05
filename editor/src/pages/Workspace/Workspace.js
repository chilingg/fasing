import ComponentsWorkspace from "./ComponentWorkspace";

export default function Workspace({ workStage }) {
    let current = <div></div>;

    switch (workStage) {
        case "components":
            current = <ComponentsWorkspace />;
            break;
    }

    return (
        <div style={{ display: "flex", flex: 1, flexDirection: "column" }}>
            {current}
        </div>
    )
}