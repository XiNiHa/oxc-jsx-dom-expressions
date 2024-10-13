#!/usr/bin/env nu

def main [repo_path: string] {
	let fixtures = ls ($'($repo_path)/packages/babel-plugin-jsx-dom-expressions/test/__*_fixtures__/*/*.js' | into glob) | $in.name
	let out = $fixtures
		| str replace -r '/(.+?)/code\.js' '/$1.js'
		| str replace -r '/(.+?)/output\.js' '/$1.expected.js'
		| str replace -r '.+/test/__(.+?)_fixtures__/(.+)' 'tests/transform/specs/$1/$2'
	$fixtures | zip $out | each {|| mkdir (dirname $in.1); cp $in.0 $in.1}
}
