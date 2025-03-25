import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ConfigProvider } from 'antd';
import fasingTheme from './theme';

ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <ConfigProvider theme={fasingTheme}>
      <App />
    </ConfigProvider>
  </React.StrictMode>,
);
