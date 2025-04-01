import { theme } from 'antd';

const fasingTheme = {
    algorithm: [theme.darkAlgorithm, theme.compactAlgorithm],
    components: {
        Splitter: {
            splitBarSize: 0
        },
        InputNumber: {
            handleWidth: 8,
        },
        Button: {
            defaultBg: '#222222',
            defaultBorderColor: '#444444'
        },
    },
    token: {
        colorPrimary: '#18bcc6',
        colorBgBase: '#222222',
        colorBgContainer: '#141414',
        colorBgElevated: '#333333',
        colorBgSolid: '#444444',

        boxShadow: '0 0 10px rgba(0, 0, 0, 0.7)',
        boxShadowSecondary: '0 0 10px rgba(0, 0, 0, 0.4)',
        containerPadding: '10px'
    },
};

export default fasingTheme;