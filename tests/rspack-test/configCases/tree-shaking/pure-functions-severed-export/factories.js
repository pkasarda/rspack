import { deepMerge } from "./merge";

export function reducerFactory(prefix) {
	return state => deepMerge(state, { type: prefix });
}

export function actionFactory(prefix) {
	return bucket => ({ type: prefix, bucket });
}
