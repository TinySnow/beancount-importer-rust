#!/usr/bin/env bash

# autopush.sh
# 此脚本用于简化 git 提交

# bash 脚本安全性保障
set -Eeuxo pipefail

if [[ $1 ]]; then

	git add .

	# 提交 message 取第二个参数，需要打引号
	git commit -m "$1"

	# 推送至 Github 远程仓库
	git push origin master

	# # 推送至 Gitee 远程仓库
	# git push gitee master
	
else
	echo "请提供参数。"
fi