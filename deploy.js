const { execSync } = require('child_process');

// Проверяем наличие токена
const token = process.env.GITHUB_TOKEN;
if (!token) {
    console.error('Ошибка: GITHUB_TOKEN не установлен');
    process.exit(1);
}

try {
    // Создаем временную директорию для публикации
    execSync('git init');
    execSync('git checkout --orphan gh-pages');
    
    // Настраиваем git
    execSync('git config --local user.email "github-actions[bot]@users.noreply.github.com"');
    execSync('git config --local user.name "github-actions[bot]"');
    
    // Убеждаемся, что мы находимся в корректной директории
    const currentDir = process.cwd();
    console.log('Текущая директория:', currentDir);
    
    // Добавляем все файлы из текущей директории
    execSync('git add .');
    
    // Создаем коммит с текущей датой
    const date = new Date().toLocaleDateString('ru-RU');
    execSync(`git commit -m "Обновление чейнджлога: ${date}"`);

    // Добавляем remote и пушим изменения
    execSync(`git remote add origin https://${token}@github.com/BuildersSC/Krevetka.git`);
    execSync('git push -f origin gh-pages');
    
    console.log('Успешно опубликовано на GitHub Pages');
} catch (error) {
    console.error('Ошибка при публикации:', error.toString());
    process.exit(1);
} 