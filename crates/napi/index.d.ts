/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export interface Config {
  moduleName?: string
  generate?: OutputType
  hydratable?: boolean
  delegateEvents?: boolean
  delegatedEvents?: Array<string>
  builtIns?: Array<string>
  requireImportSource?: boolean
  wrapConditionals?: boolean
  omitNestedClosingTags?: boolean
  contextToCustomElements?: boolean
  staticMarker?: string
  effectWrapper?: string
  memoWrapper?: string
  validate?: boolean
}
export const enum OutputType {
  Dom = 'dom'
}
export declare function transform(source: string, config?: Config | undefined | null): string
