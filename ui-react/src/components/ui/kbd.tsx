import * as React from 'react'

import { cn } from '@/lib/utils'

export interface KbdProps extends React.HTMLAttributes<HTMLElement> {}

const Kbd = React.forwardRef<HTMLElement, KbdProps>(({ className, ...props }, ref) => {
  return (
    <kbd
      ref={ref}
      className={cn(
        'pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border border-border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground opacity-100',
        className
      )}
      {...props}
    />
  )
})
Kbd.displayName = 'Kbd'

export interface KbdGroupProps extends React.HTMLAttributes<HTMLDivElement> {}

const KbdGroup = React.forwardRef<HTMLDivElement, KbdGroupProps>(({ className, ...props }, ref) => {
  return <div ref={ref} className={cn('inline-flex items-center gap-1', className)} {...props} />
})
KbdGroup.displayName = 'KbdGroup'

export { Kbd, KbdGroup }
