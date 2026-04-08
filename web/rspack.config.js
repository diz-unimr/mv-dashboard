import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default {
    entry: {
        main: './web/script.js',
    },
    output: {
        path: path.resolve(__dirname, '../resources/assets'),
        chunkFilename: '[id].js',
        sourceMap: false,
        library: {
            type: "window"
        }
    },
    module: {
        rules: [
            {
                test: /\.css$/,
                use: [{
                    loader: "postcss-loader",
                    options: {
                        postcssOptions: {
                            plugins: {
                                "@tailwindcss/postcss": {},
                            },
                        }
                    }
                }],
                type: "css"
            },
        ]
    },
    experiments: {
        css: true,
    }
}