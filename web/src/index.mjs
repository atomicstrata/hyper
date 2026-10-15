import {createSDK} from './sdk.mjs';
const sdk = createSDK({manifestURL:new URL('./renderers.json', import.meta.url)});
export const createHyperViewer = options => sdk(options);
