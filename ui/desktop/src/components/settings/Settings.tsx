import React from 'react';
import { ScrollArea } from '../ui/scroll-area';
import { Card } from '../ui/card';
import { useNavigate } from 'react-router-dom';

const EXTENSIONS_DESCRIPTION = "The Model Context Protocol (MCP) is a system that allows AI models to securely connect with local or remote resources using standard server setups. It works like a client-server setup and expands AI capabilities using three main components: Prompts, Resources, and Tools.";

export default function Settings() {
    const navigate = useNavigate();
    
    // Initialize models state from localStorage or use default values
    const [models, setModels] = React.useState(() => {
        const savedModels = localStorage.getItem('settings_models');
        return savedModels ? JSON.parse(savedModels) : {
            gpt4: false,
            gpt4lite: false,
            claude: true,
        };
    });

    // Initialize extensions state from localStorage or use default values
    const [extensions, setExtensions] = React.useState(() => {
        const savedExtensions = localStorage.getItem('settings_extensions');
        return savedExtensions ? JSON.parse(savedExtensions) : {
            fileviewer: true,
            cloudthing: true,
            mcpdice: true,
            binancedata: true,
        };
    });

    // Save models state to localStorage whenever it changes
    React.useEffect(() => {
        localStorage.setItem('settings_models', JSON.stringify(models));
    }, [models]);

    // Save extensions state to localStorage whenever it changes
    React.useEffect(() => {
        localStorage.setItem('settings_extensions', JSON.stringify(extensions));
    }, [extensions]);

    const handleModelToggle = (modelId: string) => {
        // Only allow one model to be active
        setModels(Object.keys(models).reduce((acc, key) => ({
            ...acc,
            [key]: key === modelId
        }), {}));
    };

    const handleExtensionToggle = (extensionId: string) => {
        setExtensions(prev => ({
            ...prev,
            [extensionId]: !prev[extensionId]
        }));
    };

    const handleNavClick = (section: string, e: React.MouseEvent) => {
        e.preventDefault();
        const scrollArea = document.querySelector('[data-radix-scroll-area-viewport]');
        const element = document.getElementById(section.toLowerCase());
        
        if (scrollArea && element) {
            const topPos = element.offsetTop;
            scrollArea.scrollTo({
                top: topPos,
                behavior: 'smooth'
            });
        }
    };

    const handleExit = () => {
        navigate('/chat/1', { replace: true }); // Use replace to ensure clean navigation
    };

    return (
        <div className="h-screen w-full p-[10px]">
            <Card className="h-full w-full bg-card-gradient dark:bg-dark-card-gradient border-none rounded-2xl p-6">
                <div className="h-full w-full bg-white dark:bg-gray-800 rounded-2xl overflow-hidden p-4">
                    <ScrollArea className="h-full w-full">
                        <div className="flex min-h-full">
                            {/* Left Navigation */}
                            <div className="w-48 border-r border-gray-100 dark:border-gray-700 px-6">
                                <div className="sticky top-8">
                                    <button
                                        onClick={handleExit}
                                        className="flex items-center gap-2 text-gray-600 hover:text-gray-800 
                                            dark:text-gray-400 dark:hover:text-gray-200 mb-16 mt-4"
                                    >
                                        <span className="text-xl">←</span>
                                        <span>Exit</span>
                                    </button>
                                    <div className="space-y-2">
                                        {['Models', 'Extensions', 'Keys'].map((section) => (
                                            <button
                                                key={section}
                                                onClick={(e) => handleNavClick(section, e)}
                                                className="block w-full text-left px-3 py-2 rounded-lg transition-colors
                                                    text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
                                            >
                                                {section}
                                            </button>
                                        ))}
                                    </div>
                                </div>
                            </div>

                            {/* Content Area */}
                            <div className="flex-1 px-8 py-8">
                                <div className="max-w-3xl space-y-12">
                                    {/* Models Section */}
                                    <section id="models">
                                        <div className="flex justify-between items-center mb-4">
                                            <h2 className="text-2xl font-semibold">Models</h2>
                                            <button className="text-indigo-500 hover:text-indigo-600 font-medium">
                                                Add Models
                                            </button>
                                        </div>
                                        {Object.entries(models).map(([id, enabled]) => (
                                            <div key={id} className="bg-white dark:bg-gray-800 rounded-lg p-4 mb-2">
                                                <div className="flex justify-between items-center">
                                                    <h3 className="text-lg font-medium dark:text-white">
                                                        {id === 'gpt4' ? 'GPT 4.0' :
                                                         id === 'gpt4lite' ? 'GPT 4.0 lite' : 'Claude'}
                                                    </h3>
                                                    <button 
                                                        onClick={() => handleModelToggle(id)}
                                                        className={`
                                                            relative inline-flex h-6 w-11 items-center rounded-full
                                                            ${enabled ? 'bg-indigo-500' : 'bg-gray-200 dark:bg-gray-600'}
                                                            transition-colors duration-200 ease-in-out focus:outline-none
                                                        `}
                                                    >
                                                        <span
                                                            className={`
                                                                inline-block h-5 w-5 transform rounded-full bg-white shadow
                                                                transition-transform duration-200 ease-in-out
                                                                ${enabled ? 'translate-x-[22px]' : 'translate-x-[2px]'}
                                                            `}
                                                        />
                                                    </button>
                                                </div>
                                                <p className="text-gray-500 dark:text-gray-400 text-sm mt-1">
                                                    Standard config
                                                </p>
                                            </div>
                                        ))}
                                    </section>

                                    {/* Extensions Section (formerly MCPs) */}
                                    <section id="extensions">
                                        <div className="flex justify-between items-center mb-4">
                                            <h2 className="text-2xl font-semibold">Extensions</h2>
                                            <button className="text-indigo-500 hover:text-indigo-600 font-medium">
                                                Add Extensions
                                            </button>
                                        </div>
                                        <p className="text-gray-500 dark:text-gray-400 mb-4">{EXTENSIONS_DESCRIPTION}</p>
                                        {Object.entries(extensions).map(([id, enabled]) => (
                                            <div key={id} className="bg-white dark:bg-gray-800 rounded-lg p-4 mb-2">
                                                <div className="flex justify-between items-center">
                                                    <h3 className="text-lg font-medium dark:text-white">
                                                        {id === 'fileviewer' ? 'File viewer' :
                                                         id === 'cloudthing' ? 'Cloud thing' :
                                                         id === 'mcpdice' ? 'MCP dice' : 'Binance market data'}
                                                    </h3>
                                                    <button 
                                                        onClick={() => handleExtensionToggle(id)}
                                                        className={`
                                                            relative inline-flex h-6 w-11 items-center rounded-full
                                                            ${enabled ? 'bg-indigo-500' : 'bg-gray-200 dark:bg-gray-600'}
                                                            transition-colors duration-200 ease-in-out focus:outline-none
                                                        `}
                                                    >
                                                        <span
                                                            className={`
                                                                inline-block h-5 w-5 transform rounded-full bg-white shadow
                                                                transition-transform duration-200 ease-in-out
                                                                ${enabled ? 'translate-x-[22px]' : 'translate-x-[2px]'}
                                                            `}
                                                        />
                                                    </button>
                                                </div>
                                                <p className="text-gray-500 dark:text-gray-400 text-sm mt-1">
                                                    Standard config
                                                </p>
                                            </div>
                                        ))}
                                    </section>

                                    {/* Keys Section */}
                                    <section id="keys">
                                        <div className="flex justify-between items-center mb-4">
                                            <h2 className="text-2xl font-semibold">Keys</h2>
                                            <button className="text-indigo-500 hover:text-indigo-600 font-medium">
                                                Add new key
                                            </button>
                                        </div>
                                        <p className="text-gray-500 dark:text-gray-400 mb-4">{EXTENSIONS_DESCRIPTION}</p>
                                        {['GISKey', 'AWScognito'].map(key => (
                                            <div key={key} className="bg-white dark:bg-gray-800 rounded-lg p-4 mb-2">
                                                <div className="flex justify-between items-center">
                                                    <h3 className="text-lg font-medium dark:text-white">{key}</h3>
                                                    <div className="flex items-center gap-3">
                                                        <span className="text-gray-500">{'*'.repeat(17)}</span>
                                                        <button className="text-indigo-500 hover:text-indigo-600">
                                                            ✏️
                                                        </button>
                                                    </div>
                                                </div>
                                            </div>
                                        ))}
                                    </section>
                                </div>
                            </div>
                        </div>
                    </ScrollArea>
                </div>
            </Card>
        </div>
    );
} 