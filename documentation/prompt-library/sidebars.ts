import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  promptSidebar: [
    {
      type: 'doc',
      id: 'index',
      label: 'Home',
    },
    {
      type: 'category',
      label: 'Prompts',
      items: [
        {
          type: 'category',
          label: 'Developer',
          items: ['prompts/developer/index'],
        },
        {
          type: 'category',
          label: 'Business',
          items: ['prompts/business/index'],
        },
      ],
    },
    {
      type: 'category',
      label: 'GooseHints',
      items: [
        {
          type: 'category',
          label: 'Developer',
          items: ['goosehints/developer/index'],
        },
        {
          type: 'category',
          label: 'Business',
          items: ['goosehints/business/index'],
        },
      ],
    },
  ],
};

export default sidebars;