import Cookies from 'js-cookie';
import {AppId} from "../fluence/apps";
import {DeployableAppId} from "../fluence/deployable";

export interface DeployedAppSaved {
    deployedAppId: AppId,
    deployedAppTypeId: DeployableAppId
}

export function getDeployedApp(): DeployedAppSaved | null {
    const deployedAppId = Cookies.get('deployedAppId');
    const deployedAppTypeId = Cookies.get('deployedAppTypeId');
    if (deployedAppId && deployedAppTypeId) {
        return {
            deployedAppId: deployedAppId,
            deployedAppTypeId: deployedAppTypeId
        };
    }

    return null;
}

export function clearDeployedApp(): void {
    Cookies.remove('deployedAppId');
    Cookies.remove('deployedAppTypeId');
}

export function saveDeployedApp(deployedAppId: AppId, deployedAppTypeId: DeployableAppId): void {
    clearDeployedApp();
    Cookies.set('deployedAppId', deployedAppId, { expires: 365 });
    Cookies.set('deployedAppTypeId', deployedAppTypeId, { expires: 365 });
}