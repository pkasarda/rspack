import { actionFactory, reducerFactory } from "./factories";

export const mergeStore = actionFactory("Store/MERGE");

export const storeReducers = {
	store: reducerFactory("Store/MERGE")
};
