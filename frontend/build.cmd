@echo off
setlocal

cd %~dp0

npm install
xcopy /y node_modules\@primer\css\dist\primer.css ..\src\
