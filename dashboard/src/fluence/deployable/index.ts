import {App} from "..";

export type DeployableAppId = string;

export interface DeployableApp {
    name: string,
    storage_hash: string,
    cluster_size: number,
}

export async function deploy(app: DeployableApp): Promise<App> {
    // TODO
    throw new Error("not implemented");
}

export const deployableAppIds: [DeployableAppId] = ["llamadb"];

export const deployableApps: {[key: string]: DeployableApp} = {
    "llamadb": {
        name: "SQL DB (llamadb)",
        storage_hash: "0x9918b8657755b41096da7a7da0528550ffce4a812c2295d2811c86d29be23326",
        cluster_size: 4
    }
};