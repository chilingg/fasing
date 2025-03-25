import "../App.css";
import React from "react";
import ReactDOM from "react-dom/client";

import { ConfigProvider } from 'antd';

import fasingTheme from '../theme';
import Editor from "./Editor";

ReactDOM.createRoot(document.getElementById("root")).render(
    <React.StrictMode>
        <ConfigProvider theme={fasingTheme}>
            <Editor />
        </ConfigProvider>
    </React.StrictMode>,
);
