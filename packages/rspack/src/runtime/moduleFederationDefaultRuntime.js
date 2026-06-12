// @ts-nocheck
var __module_federation_bundler_runtime__,
  __module_federation_runtime_plugins__,
  __module_federation_remote_infos__,
  __module_federation_container_name__,
  __module_federation_share_strategy__,
  __module_federation_share_fallbacks__,
  __module_federation_library_type__;
export default function () {
  if (
    (__webpack_require__.initializeSharingData ||
      __webpack_require__.initializeExposesData) &&
    __webpack_require__.federation
  ) {
    const override = (obj, key, value) => {
      if (!obj) return;
      if (obj[key]) obj[key] = value;
    };
    const merge = (obj, key, fn) => {
      const value = fn();
      if (Array.isArray(value)) {
        obj[key] ??= [];
        obj[key].push(...value);
      } else if (typeof value === 'object' && value !== null) {
        obj[key] ??= {};
        Object.assign(obj[key], value);
      }
    };
    const early = (obj, key, initial) => {
      obj[key] ??= initial();
    };
    const remotesLoadingChunkMapping =
      __webpack_require__.remotesLoadingData?.chunkMapping ?? {};
    const remotesLoadingModuleIdToRemoteDataMapping =
      __webpack_require__.remotesLoadingData?.moduleIdToRemoteDataMapping ?? {};
    const initializeSharingScopeToInitDataMapping =
      __webpack_require__.initializeSharingData?.scopeToSharingDataMapping ??
      {};
    const consumesLoadingChunkMapping =
      __webpack_require__.consumesLoadingData?.chunkMapping ?? {};
    const consumesLoadingModuleToConsumeDataMapping =
      __webpack_require__.consumesLoadingData?.moduleIdToConsumeDataMapping ??
      {};
    const consumesLoadinginstalledModules = {};
    const initializeSharingInitPromises = [];
    const initializeSharingInitTokens = {};
    const containerShareScope =
      __webpack_require__.initializeExposesData?.shareScope;
    const toList = (value) => (Array.isArray(value) ? value : []);
    const pushUnique = (target, values) => {
      for (const value of values) {
        if (value !== undefined && value !== null && !target.includes(value)) {
          target.push(value);
        }
      }
    };
    const getAffectedConsumerModuleIds = (remoteModuleIds) => {
      const consumerModuleIds = [];
      const queue = [];
      const consumerMapping =
        __webpack_require__.remotesLoadingData
          ?.remoteModuleIdToConsumerModuleIds ?? {};
      const parentMapping =
        __webpack_require__.remotesLoadingData
          ?.consumerModuleIdToParentModuleIds ?? {};

      for (const remoteModuleId of remoteModuleIds) {
        pushUnique(queue, toList(consumerMapping[remoteModuleId]));
      }
      for (let i = 0; i < queue.length; i++) {
        const consumerModuleId = queue[i];
        if (consumerModuleIds.includes(consumerModuleId)) continue;
        consumerModuleIds.push(consumerModuleId);
        pushUnique(queue, toList(parentMapping[consumerModuleId]));
      }
      return consumerModuleIds;
    };
    const deleteModuleCache = (moduleIds) => {
      for (const moduleId of moduleIds) {
        delete __webpack_require__.c[moduleId];
      }
    };
    const clearRemoteCache = (options) => {
      const name = typeof options === 'string' ? options : options?.name;
      if (!name) {
        return Promise.reject(
          new Error('clearRemoteCache requires a remote name'),
        );
      }

      const remoteKey =
        typeof options === 'object' && options
          ? options.remoteKey || name
          : name;
      const remotesLoadingData = __webpack_require__.remotesLoadingData ?? {};
      const idToExternalAndNameMapping =
        __webpack_require__.federation.bundlerRuntimeOptions.remotes
          .idToExternalAndNameMapping ?? {};
      const remoteModuleIds = [];

      pushUnique(
        remoteModuleIds,
        toList(remotesLoadingData.remoteKeyToRemoteModuleIds?.[remoteKey]),
      );
      if (remoteModuleIds.length === 0) {
        for (const [moduleId, data] of Object.entries(
          remotesLoadingModuleIdToRemoteDataMapping,
        )) {
          if (data.remoteName === remoteKey || data.remoteName === name) {
            remoteModuleIds.push(moduleId);
          }
        }
      }
      if (remoteModuleIds.length === 0) {
        return Promise.reject(
          new Error(`Cannot find remote "${name}" in remote loading data`),
        );
      }

      const externalModuleIds = [];
      pushUnique(
        externalModuleIds,
        toList(remotesLoadingData.remoteKeyToExternalModuleIds?.[remoteKey]),
      );
      for (const remoteModuleId of remoteModuleIds) {
        const data = remotesLoadingModuleIdToRemoteDataMapping[remoteModuleId];
        if (data) {
          pushUnique(externalModuleIds, [data.externalModuleId]);
        }
      }

      const pendingRemoteLoads = [];
      for (const remoteModuleId of remoteModuleIds) {
        for (const data of [
          remotesLoadingModuleIdToRemoteDataMapping[remoteModuleId],
          idToExternalAndNameMapping[remoteModuleId],
        ]) {
          if (data?.p && typeof data.p.then === 'function') {
            pendingRemoteLoads.push(data.p.catch(() => {}));
          }
        }
      }

      return Promise.all(pendingRemoteLoads).then(() => {
        const consumerModuleIds = getAffectedConsumerModuleIds(remoteModuleIds);
        for (const remoteModuleId of remoteModuleIds) {
          const data =
            remotesLoadingModuleIdToRemoteDataMapping[remoteModuleId];
          const runtimeData = idToExternalAndNameMapping[remoteModuleId];
          if (data) delete data.p;
          if (runtimeData) delete runtimeData.p;
          delete __webpack_require__.m[remoteModuleId];
        }
        deleteModuleCache(remoteModuleIds);
        deleteModuleCache(externalModuleIds);
        deleteModuleCache(consumerModuleIds);

        const instance = __webpack_require__.federation.instance;
        if (instance) {
          const remoteNames = [name, remoteKey];
          for (const remoteInfo of toList(
            __webpack_require__.federation.bundlerRuntimeOptions.remotes
              .remoteInfos?.[remoteKey],
          )) {
            pushUnique(remoteNames, [remoteInfo.name, remoteInfo.alias]);
          }
          for (const remoteName of remoteNames) {
            instance.moduleCache?.delete(remoteName);
          }
          const idToRemoteMap = instance.remoteHandler?.idToRemoteMap;
          if (idToRemoteMap) {
            for (const [id, remote] of Object.entries(idToRemoteMap)) {
              if (
                remoteNames.includes(remote.name) ||
                remoteNames.some((remoteName) => id.startsWith(remoteName))
              ) {
                delete idToRemoteMap[id];
              }
            }
          }
        }

        return {
          name,
          cleared: true,
        };
      });
    };

    for (const key in __module_federation_bundler_runtime__) {
      __webpack_require__.federation[key] =
        __module_federation_bundler_runtime__[key];
    }

    early(
      __webpack_require__.federation,
      'libraryType',
      () => __module_federation_library_type__,
    );
    early(
      __webpack_require__.federation,
      'sharedFallback',
      () => __module_federation_share_fallbacks__,
    );
    const sharedFallback = __webpack_require__.federation.sharedFallback;
    early(
      __webpack_require__.federation,
      'consumesLoadingModuleToHandlerMapping',
      () => {
        const consumesLoadingModuleToHandlerMapping = {};
        for (let [moduleId, data] of Object.entries(
          consumesLoadingModuleToConsumeDataMapping,
        )) {
          consumesLoadingModuleToHandlerMapping[moduleId] = {
            getter: sharedFallback
              ? __webpack_require__.federation.bundlerRuntime?.getSharedFallbackGetter(
                  {
                    shareKey: data.shareKey,
                    factory: data.fallback,
                    webpackRequire: __webpack_require__,
                    libraryType: __webpack_require__.federation.libraryType,
                  },
                )
              : data.fallback,
            treeShakingGetter: sharedFallback ? data.fallback : undefined,
            shareInfo: {
              shareConfig: {
                fixedDependencies: false,
                requiredVersion: data.requiredVersion,
                strictVersion: data.strictVersion,
                singleton: data.singleton,
                eager: data.eager,
              },
              scope: [data.shareScope],
            },
            shareKey: data.shareKey,
            treeShaking: __webpack_require__.federation.sharedFallback
              ? {
                  get: data.fallback,
                  mode: data.treeShakingMode,
                }
              : undefined,
          };
        }
        return consumesLoadingModuleToHandlerMapping;
      },
    );

    early(__webpack_require__.federation, 'initOptions', () => ({}));
    early(
      __webpack_require__.federation.initOptions,
      'name',
      () => __module_federation_container_name__,
    );
    early(
      __webpack_require__.federation.initOptions,
      'shareStrategy',
      () => __module_federation_share_strategy__,
    );
    early(__webpack_require__.federation.initOptions, 'shared', () => {
      const shared = {};
      for (let [scope, stages] of Object.entries(
        initializeSharingScopeToInitDataMapping,
      )) {
        for (let stage of stages) {
          if (typeof stage === 'object' && stage !== null) {
            const {
              name,
              version,
              factory,
              eager,
              singleton,
              requiredVersion,
              strictVersion,
              treeShakingMode,
            } = stage;
            const shareConfig = {};
            const isValidValue = function (val) {
              return typeof val !== 'undefined';
            };
            if (isValidValue(singleton)) {
              shareConfig.singleton = singleton;
            }
            if (isValidValue(requiredVersion)) {
              shareConfig.requiredVersion = requiredVersion;
            }
            if (isValidValue(eager)) {
              shareConfig.eager = eager;
            }
            if (isValidValue(strictVersion)) {
              shareConfig.strictVersion = strictVersion;
            }
            const options = {
              version,
              scope: [scope],
              shareConfig,
              get: factory,
              treeShaking: treeShakingMode
                ? {
                    mode: treeShakingMode,
                  }
                : undefined,
            };
            if (shared[name]) {
              shared[name].push(options);
            } else {
              shared[name] = [options];
            }
          }
        }
      }
      return shared;
    });
    merge(__webpack_require__.federation.initOptions, 'remotes', () =>
      Object.values(__module_federation_remote_infos__)
        .flat()
        .filter((remote) => remote.externalType === 'script'),
    );
    merge(
      __webpack_require__.federation.initOptions,
      'plugins',
      () => __module_federation_runtime_plugins__,
    );

    early(__webpack_require__.federation, 'bundlerRuntimeOptions', () => ({}));
    early(
      __webpack_require__.federation.bundlerRuntimeOptions,
      'remotes',
      () => ({}),
    );
    early(
      __webpack_require__.federation.bundlerRuntimeOptions.remotes,
      'chunkMapping',
      () => remotesLoadingChunkMapping,
    );
    early(
      __webpack_require__.federation.bundlerRuntimeOptions.remotes,
      'remoteInfos',
      () => __module_federation_remote_infos__,
    );
    early(
      __webpack_require__.federation.bundlerRuntimeOptions.remotes,
      'idToExternalAndNameMapping',
      () => {
        const remotesLoadingIdToExternalAndNameMappingMapping = {};
        for (let [moduleId, data] of Object.entries(
          remotesLoadingModuleIdToRemoteDataMapping,
        )) {
          remotesLoadingIdToExternalAndNameMappingMapping[moduleId] = [
            data.shareScope,
            data.name,
            data.externalModuleId,
            data.remoteName,
          ];
        }
        return remotesLoadingIdToExternalAndNameMappingMapping;
      },
    );
    early(
      __webpack_require__.federation.bundlerRuntimeOptions.remotes,
      'webpackRequire',
      () => __webpack_require__,
    );
    merge(
      __webpack_require__.federation.bundlerRuntimeOptions.remotes,
      'idToRemoteMap',
      () => {
        const idToRemoteMap = {};
        for (let [id, remoteData] of Object.entries(
          remotesLoadingModuleIdToRemoteDataMapping,
        )) {
          const info =
            __module_federation_remote_infos__[remoteData.remoteName];
          if (info) idToRemoteMap[id] = info;
        }
        return idToRemoteMap;
      },
    );

    override(
      __webpack_require__,
      'S',
      __webpack_require__.federation.bundlerRuntime.S,
    );
    if (__webpack_require__.federation.attachShareScopeMap) {
      __webpack_require__.federation.attachShareScopeMap(__webpack_require__);
    }

    override(__webpack_require__.f, 'remotes', (chunkId, promises) =>
      __webpack_require__.federation.bundlerRuntime.remotes({
        chunkId,
        promises,
        chunkMapping: remotesLoadingChunkMapping,
        idToExternalAndNameMapping:
          __webpack_require__.federation.bundlerRuntimeOptions.remotes
            .idToExternalAndNameMapping,
        idToRemoteMap:
          __webpack_require__.federation.bundlerRuntimeOptions.remotes
            .idToRemoteMap,
        webpackRequire: __webpack_require__,
      }),
    );
    override(__webpack_require__.f, 'consumes', (chunkId, promises) =>
      __webpack_require__.federation.bundlerRuntime.consumes({
        chunkId,
        promises,
        chunkMapping: consumesLoadingChunkMapping,
        moduleToHandlerMapping:
          __webpack_require__.federation.consumesLoadingModuleToHandlerMapping,
        installedModules: consumesLoadinginstalledModules,
        webpackRequire: __webpack_require__,
      }),
    );
    override(__webpack_require__, 'I', (name, initScope) =>
      __webpack_require__.federation.bundlerRuntime.I({
        shareScopeName: name,
        initScope,
        initPromises: initializeSharingInitPromises,
        initTokens: initializeSharingInitTokens,
        webpackRequire: __webpack_require__,
      }),
    );
    override(
      __webpack_require__,
      'initContainer',
      (shareScope, initScope, remoteEntryInitOptions) =>
        __webpack_require__.federation.bundlerRuntime.initContainerEntry({
          shareScope,
          initScope,
          remoteEntryInitOptions,
          shareScopeKey: containerShareScope,
          webpackRequire: __webpack_require__,
        }),
    );
    override(__webpack_require__, 'getContainer', (module, getScope) => {
      var moduleMap = __webpack_require__.initializeExposesData.moduleMap;
      __webpack_require__.R = getScope;
      getScope = Object.prototype.hasOwnProperty.call(moduleMap, module)
        ? moduleMap[module]()
        : Promise.resolve().then(() => {
            throw new Error(
              'Module "' + module + '" does not exist in container.',
            );
          });
      __webpack_require__.R = undefined;
      return getScope;
    });

    __webpack_require__.federation.instance =
      __webpack_require__.federation.bundlerRuntime.init({
        webpackRequire: __webpack_require__,
      });
    __webpack_require__.federation.clearRemoteCache = clearRemoteCache;
    if (
      typeof __webpack_require__.federation.instance.clearCache !== 'function'
    ) {
      __webpack_require__.federation.instance.clearCache = clearRemoteCache;
    }

    if (__webpack_require__.consumesLoadingData?.initialConsumes) {
      __webpack_require__.federation.bundlerRuntime.installInitialConsumes({
        webpackRequire: __webpack_require__,
        installedModules: consumesLoadinginstalledModules,
        initialConsumes:
          __webpack_require__.consumesLoadingData.initialConsumes,
        moduleToHandlerMapping:
          __webpack_require__.federation.consumesLoadingModuleToHandlerMapping,
      });
    }
  }
}
