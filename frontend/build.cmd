@echo off
setlocal

cd %~dp0

npm install

xcopy /y node_modules\@primer\css\dist\primer.css ..\src\
xcopy /y node_modules\@primer\css\dist\primer.css.map ..\src\

xcopy /y node_modules\@primer\css\dist\primer.css ..\site\css\
xcopy /y node_modules\@primer\css\dist\primer.css.map ..\site\css\
